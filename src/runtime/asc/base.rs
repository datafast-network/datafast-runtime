use crate::impl_asc_type;

use crate::errors::AscError;
use semver::Version;
use std::fmt;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

pub const SIZE_OF_RT_SIZE: u32 = 4;
pub const HEADER_SIZE: usize = 20;
pub const MAX_RECURSION_DEPTH: usize = 128;

pub trait AscIndexId {
    /// Constant string with the name of the type in AssemblyScript.
    /// This is used to get the identifier for the type in memory layout.
    /// Info about memory layout:
    /// https://www.assemblyscript.org/memory.html#common-header-layout.
    /// Info about identifier (`idof<T>`):
    /// https://www.assemblyscript.org/garbage-collection.html#runtime-interface
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId;
}

pub trait AscHeap {
    /// Allocate new space and write `bytes`, return the allocated address.
    fn raw_new(&mut self, bytes: &[u8]) -> Result<u32, AscError>;

    fn read<'a>(
        &self,
        offset: u32,
        buffer: &'a mut [MaybeUninit<u8>],
    ) -> Result<&'a mut [u8], AscError>;

    fn read_u32(&self, offset: u32) -> Result<u32, AscError>;

    fn api_version(&self) -> Version;

    fn asc_type_id(&mut self, type_id_index: IndexForAscTypeId) -> Result<u32, AscError>;
}

pub struct AscPtr<C>(u32, PhantomData<C>);

impl<T> Copy for AscPtr<T> {}

impl<T> Clone for AscPtr<T> {
    fn clone(&self) -> Self {
        AscPtr(self.0, PhantomData)
    }
}

impl<T> Default for AscPtr<T> {
    fn default() -> Self {
        AscPtr(0, PhantomData)
    }
}

impl<T> fmt::Debug for AscPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<C> AscPtr<C> {
    /// A raw pointer to be passed to Wasm.
    pub fn wasm_ptr(self) -> u32 {
        self.0
    }

    #[inline(always)]
    pub fn new(heap_ptr: u32) -> Self {
        Self(heap_ptr, PhantomData)
    }
}

/// A type that has a direct correspondence to an Asc type.
///
/// This can be derived for structs that are `#[repr(C)]`, contain no padding
/// and whose fields are all `AscValue`. Enums can derive if they are `#[repr(u32)]`.
///
/// Special classes like `ArrayBuffer` use custom impls.
///
/// See https://github.com/graphprotocol/graph-node/issues/607 for more considerations.
/// A type that has a direct correspondence to an Asc type.
///
/// This can be derived for structs that are `#[repr(C)]`, contain no padding
/// and whose fields are all `AscValue`. Enums can derive if they are `#[repr(u32)]`.
///
/// Special classes like `ArrayBuffer` use custom impls.
///
/// See https://github.com/graphprotocol/graph-node/issues/607 for more considerations.
pub trait AscType: Sized {
    /// Transform the Rust representation of this instance into an sequence of
    /// bytes that is precisely the memory layout of a corresponding Asc instance.
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError>;

    /// The Rust representation of an Asc object as layed out in Asc memory.
    fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, AscError>;

    fn content_len(&self, asc_bytes: &[u8]) -> usize {
        asc_bytes.len()
    }

    /// Size of the corresponding Asc instance in bytes.
    /// Only used for version <= 0.0.3.
    fn asc_size<H: AscHeap + ?Sized>(_ptr: AscPtr<Self>, _heap: &H) -> Result<u32, AscError> {
        Ok(std::mem::size_of::<Self>() as u32)
    }
}

// Only implemented because of structs that derive AscType and
// contain fields that are PhantomData.
impl<T> AscType for PhantomData<T> {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        Ok(vec![])
    }

    fn from_asc_bytes(asc_obj: &[u8], _api_version: &Version) -> Result<Self, AscError> {
        assert!(asc_obj.is_empty());

        Ok(Self)
    }
}

impl<C: AscType> AscPtr<C> {
    /// Create a pointer that is equivalent to AssemblyScript's `null`.
    #[inline(always)]
    pub fn null() -> Self {
        AscPtr::new(0)
    }

    /// Read from `self` into the Rust struct `C`.
    pub fn read_ptr<H: AscHeap + ?Sized>(self, heap: &H) -> Result<C, AscError> {
        let len = match heap.api_version() {
            // TODO: The version check here conflicts with the comment on C::asc_size,
            // which states "Only used for version <= 0.0.3."
            version if version <= Version::new(0, 0, 4) => C::asc_size(self, heap),
            _ => self.read_len(heap),
        }?;

        let using_buffer = |buffer: &mut [MaybeUninit<u8>]| {
            let buffer = heap.read(self.0, buffer)?;
            C::from_asc_bytes(buffer, &heap.api_version())
        };

        let len = len as usize;

        if len <= 32 {
            let mut buffer = [MaybeUninit::<u8>::uninit(); 32];
            using_buffer(&mut buffer[..len])
        } else {
            let mut buffer = Vec::with_capacity(len);
            using_buffer(buffer.spare_capacity_mut())
        }
    }

    /// Allocate `asc_obj` as an Asc object of class `C`.
    pub fn alloc_obj<H: AscHeap + ?Sized>(asc_obj: C, heap: &mut H) -> Result<AscPtr<C>, AscError>
    where
        C: AscIndexId,
    {
        match heap.api_version() {
            version if version <= Version::new(0, 0, 4) => {
                let heap_ptr = heap.raw_new(&asc_obj.to_asc_bytes()?)?;
                Ok(AscPtr::new(heap_ptr))
            }
            _ => {
                let mut bytes = asc_obj.to_asc_bytes()?;

                let aligned_len = padding_to_16(bytes.len());
                // Since AssemblyScript keeps all allocated objects with a 16 byte alignment,
                // we need to do the same when we allocate ourselves.
                bytes.extend(std::iter::repeat(0).take(aligned_len));

                let header = Self::generate_header(
                    heap,
                    C::INDEX_ASC_TYPE_ID,
                    asc_obj.content_len(&bytes),
                    bytes.len(),
                )?;
                let header_len = header.len() as u32;

                let heap_ptr = heap.raw_new(&[header, bytes].concat())?;

                // Use header length as offset. so the AscPtr points directly at the content.
                Ok(AscPtr::new(heap_ptr + header_len))
            }
        }
    }

    /// Helper used by arrays and strings to read their length.
    /// Only used for version <= 0.0.4.
    pub fn read_u32<H: AscHeap + ?Sized>(&self, heap: &H) -> Result<u32, AscError> {
        // Read the bytes pointed to by `self` as the bytes of a `u32`.
        heap.read_u32(self.0)
    }

    /// Helper that generates an AssemblyScript header.
    /// An AssemblyScript header has 20 bytes and it is composed of 5 values.
    /// - mm_info: usize -> size of all header contents + payload contents + padding
    /// - gc_info: usize -> first GC info (we don't free memory so it's irrelevant)
    /// - gc_info2: usize -> second GC info (we don't free memory so it's irrelevant)
    /// - rt_id: u32 -> identifier for the class being allocated
    /// - rt_size: u32 -> content size
    /// Only used for version >= 0.0.5.
    fn generate_header<H: AscHeap + ?Sized>(
        heap: &mut H,
        type_id_index: IndexForAscTypeId,
        content_length: usize,
        full_length: usize,
    ) -> Result<Vec<u8>, AscError> {
        let mut header: Vec<u8> = Vec::with_capacity(20);

        let gc_info: [u8; 4] = (0u32).to_le_bytes();
        let gc_info2: [u8; 4] = (0u32).to_le_bytes();
        let asc_type_id = heap.asc_type_id(type_id_index)?;
        let rt_id: [u8; 4] = asc_type_id.to_le_bytes();
        let rt_size: [u8; 4] = (content_length as u32).to_le_bytes();

        let mm_info: [u8; 4] =
            ((gc_info.len() + gc_info2.len() + rt_id.len() + rt_size.len() + full_length) as u32)
                .to_le_bytes();

        header.extend(mm_info);
        header.extend(gc_info);
        header.extend(gc_info2);
        header.extend(rt_id);
        header.extend(rt_size);

        Ok(header)
    }

    /// Helper to read the length from the header.
    /// An AssemblyScript header has 20 bytes, and it's right before the content, and composed by:
    /// - mm_info: usize
    /// - gc_info: usize
    /// - gc_info2: usize
    /// - rt_id: u32
    /// - rt_size: u32
    /// This function returns the `rt_size`.
    /// Only used for version >= 0.0.5.
    pub fn read_len<H: AscHeap + ?Sized>(&self, heap: &H) -> Result<u32, AscError> {
        // We're trying to read the pointer below, we should check it's
        // not null before using it.
        self.check_is_not_null()?;

        let start_of_rt_size = self
            .0
            .checked_sub(SIZE_OF_RT_SIZE)
            .ok_or(AscError::Overflow(self.0))?;

        heap.read_u32(start_of_rt_size)
    }

    /// Conversion to `u64` for use with `AscEnum`.
    pub fn as_payload(&self) -> u64 {
        self.0 as u64
    }

    /// We typically assume `AscPtr` is never null, but for types such as `string | null` it can be.
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// There's no problem in an AscPtr being 'null' (see above AscPtr::is_null function).
    /// However if one tries to read that pointer, it should fail with a helpful error message,
    /// this function does this error handling.
    ///
    /// Summary: ALWAYS call this before reading an AscPtr.
    pub fn check_is_not_null(&self) -> Result<(), AscError> {
        if self.is_null() {
            return Err(AscError::Plain("Tried to read AssemblyScript value that is 'null'. Suggestion: look into the function that the error happened and add 'log' calls till you find where a 'null' value is being used as non-nullable. It's likely that you're calling a 'graph-ts' function (or operator) with a 'null' value when it doesn't support it.".to_string()));
        }

        Ok(())
    }

    // Erase type information.
    pub fn erase(self) -> AscPtr<()> {
        AscPtr::new(self.0)
    }
}

impl<C> From<u32> for AscPtr<C> {
    fn from(ptr: u32) -> Self {
        AscPtr::new(ptr)
    }
}

impl<T> AscType for AscPtr<T> {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        self.0.to_asc_bytes()
    }

    fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, AscError> {
        let bytes = u32::from_asc_bytes(asc_obj, api_version)?;
        Ok(AscPtr::new(bytes))
    }
}

/// An Asc primitive or an `AscPtr` into the Asc heap. A type marked as
/// `AscValue` must have the same byte representation in Rust and Asc, including
/// same size, and size must be equal to alignment.
pub trait AscValue: AscType + Copy + Default {}

impl<T> AscValue for AscPtr<T> {}
impl AscValue for bool {}

impl AscType for bool {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        Ok(vec![*self as u8])
    }

    fn from_asc_bytes(asc_obj: &[u8], _api_version: &Version) -> Result<Self, AscError> {
        if asc_obj.len() != 1 {
            Err(AscError::IncorrectBool(asc_obj.len()))
        } else {
            Ok(asc_obj[0] != 0)
        }
    }
}

impl_asc_type!(u8, u16, u32, u64, i8, i32, i64, f32, f64);

// /// Contains type IDs and their discriminants for every blockchain supported by Graph-Node.
// ///
// /// Each variant corresponds to the unique ID of an AssemblyScript concrete class used in the
// /// [`runtime`].
// ///
// /// # Rules for updating this enum
// ///
// /// 1 .The discriminants must have the same value as their counterparts in `TypeId` enum from
// ///    graph-ts' `global` module. If not, the runtime will fail to determine the correct class
// ///    during allocation.
// /// 2. Each supported blockchain has a reserved space of 1,000 contiguous variants.
// /// 3. Once defined, items and their discriminants cannot be changed, as this would break running
// ///    subgraphs compiled in previous versions of this representation.
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub enum IndexForAscTypeId {
    // Ethereum type IDs
    String = 0,
    ArrayBuffer = 1,
    Int8Array = 2,
    Int16Array = 3,
    Int32Array = 4,
    Int64Array = 5,
    Uint8Array = 6,
    Uint16Array = 7,
    Uint32Array = 8,
    Uint64Array = 9,
    Float32Array = 10,
    Float64Array = 11,
    BigDecimal = 12,
    ArrayBool = 13,
    ArrayUint8Array = 14,
    ArrayEthereumValue = 15,
    ArrayStoreValue = 16,
    ArrayJsonValue = 17,
    ArrayString = 18,
    ArrayEventParam = 19,
    ArrayTypedMapEntryStringJsonValue = 20,
    ArrayTypedMapEntryStringStoreValue = 21,
    SmartContractCall = 22,
    EventParam = 23,
    EthereumTransaction = 24,
    EthereumBlock = 25,
    EthereumCall = 26,
    WrappedTypedMapStringJsonValue = 27,
    WrappedBool = 28,
    WrappedJsonValue = 29,
    EthereumValue = 30,
    StoreValue = 31,
    JsonValue = 32,
    EthereumEvent = 33,
    TypedMapEntryStringStoreValue = 34,
    TypedMapEntryStringJsonValue = 35,
    TypedMapStringStoreValue = 36,
    TypedMapStringJsonValue = 37,
    TypedMapStringTypedMapStringJsonValue = 38,
    ResultTypedMapStringJsonValueBool = 39,
    ResultJsonValueBool = 40,
    ArrayU8 = 41,
    ArrayU16 = 42,
    ArrayU32 = 43,
    ArrayU64 = 44,
    ArrayI8 = 45,
    ArrayI16 = 46,
    ArrayI32 = 47,
    ArrayI64 = 48,
    ArrayF32 = 49,
    ArrayF64 = 50,
    ArrayBigDecimal = 51,

    // Near Type IDs
    NearArrayDataReceiver = 52,
    NearArrayCryptoHash = 53,
    NearArrayActionEnum = 54,
    NearArrayMerklePathItem = 55,
    NearArrayValidatorStake = 56,
    NearArraySlashedValidator = 57,
    NearArraySignature = 58,
    NearArrayChunkHeader = 59,
    NearAccessKeyPermissionEnum = 60,
    NearActionEnum = 61,
    NearDirectionEnum = 62,
    NearPublicKey = 63,
    NearSignature = 64,
    NearFunctionCallPermission = 65,
    NearFullAccessPermission = 66,
    NearAccessKey = 67,
    NearDataReceiver = 68,
    NearCreateAccountAction = 69,
    NearDeployContractAction = 70,
    NearFunctionCallAction = 71,
    NearTransferAction = 72,
    NearStakeAction = 73,
    NearAddKeyAction = 74,
    NearDeleteKeyAction = 75,
    NearDeleteAccountAction = 76,
    NearActionReceipt = 77,
    NearSuccessStatusEnum = 78,
    NearMerklePathItem = 79,
    NearExecutionOutcome = 80,
    NearSlashedValidator = 81,
    NearBlockHeader = 82,
    NearValidatorStake = 83,
    NearChunkHeader = 84,
    NearBlock = 85,
    NearReceiptWithOutcome = 86,
    // Reserved discriminant space for more Near type IDs: [87, 999]:
    // Continue to add more Near type IDs here.
    // e.g.:
    // NextNearType = 87,
    // AnotherNearType = 88,
    // ...
    // LastNearType = 999,

    // Reserved discriminant space for more Ethereum type IDs: [1000, 1499]
    TransactionReceipt = 1000,
    Log = 1001,
    ArrayH256 = 1002,
    ArrayLog = 1003,
    ArrayTypedMapStringStoreValue = 1004,
    ArrayEthereumTransaction = 1005,
    // Continue to add more Ethereum type IDs here.
    // e.g.:
    // NextEthereumType = 1004,
    // AnotherEthereumType = 1005,
    // ...
    // LastEthereumType = 1499,

    // Reserved discriminant space for Cosmos type IDs: [1,500, 2,499]
    CosmosAny = 1500,
    CosmosAnyArray = 1501,
    CosmosBytesArray = 1502,
    CosmosCoinArray = 1503,
    CosmosCommitSigArray = 1504,
    CosmosEventArray = 1505,
    CosmosEventAttributeArray = 1506,
    CosmosEvidenceArray = 1507,
    CosmosModeInfoArray = 1508,
    CosmosSignerInfoArray = 1509,
    CosmosTxResultArray = 1510,
    CosmosValidatorArray = 1511,
    CosmosValidatorUpdateArray = 1512,
    CosmosAuthInfo = 1513,
    CosmosBlock = 1514,
    CosmosBlockId = 1515,
    CosmosBlockIdFlagEnum = 1516,
    CosmosBlockParams = 1517,
    CosmosCoin = 1518,
    CosmosCommit = 1519,
    CosmosCommitSig = 1520,
    CosmosCompactBitArray = 1521,
    CosmosConsensus = 1522,
    CosmosConsensusParams = 1523,
    CosmosDuplicateVoteEvidence = 1524,
    CosmosDuration = 1525,
    CosmosEvent = 1526,
    CosmosEventAttribute = 1527,
    CosmosEventData = 1528,
    CosmosEventVote = 1529,
    CosmosEvidence = 1530,
    CosmosEvidenceList = 1531,
    CosmosEvidenceParams = 1532,
    CosmosFee = 1533,
    CosmosHeader = 1534,
    CosmosHeaderOnlyBlock = 1535,
    CosmosLightBlock = 1536,
    CosmosLightClientAttackEvidence = 1537,
    CosmosModeInfo = 1538,
    CosmosModeInfoMulti = 1539,
    CosmosModeInfoSingle = 1540,
    CosmosPartSetHeader = 1541,
    CosmosPublicKey = 1542,
    CosmosResponseBeginBlock = 1543,
    CosmosResponseDeliverTx = 1544,
    CosmosResponseEndBlock = 1545,
    CosmosSignModeEnum = 1546,
    CosmosSignedHeader = 1547,
    CosmosSignedMsgTypeEnum = 1548,
    CosmosSignerInfo = 1549,
    CosmosTimestamp = 1550,
    CosmosTip = 1551,
    CosmosTransactionData = 1552,
    CosmosTx = 1553,
    CosmosTxBody = 1554,
    CosmosTxResult = 1555,
    CosmosValidator = 1556,
    CosmosValidatorParams = 1557,
    CosmosValidatorSet = 1558,
    CosmosValidatorSetUpdates = 1559,
    CosmosValidatorUpdate = 1560,
    CosmosVersionParams = 1561,
    CosmosMessageData = 1562,
    CosmosTransactionContext = 1563,
    // Continue to add more Cosmos type IDs here.
    // e.g.:
    // NextCosmosType = 1564,
    // AnotherCosmosType = 1565,
    // ...
    // LastCosmosType = 2499,

    // Arweave types
    ArweaveBlock = 2500,
    ArweaveProofOfAccess = 2501,
    ArweaveTag = 2502,
    ArweaveTagArray = 2503,
    ArweaveTransaction = 2504,
    ArweaveTransactionArray = 2505,
    ArweaveTransactionWithBlockPtr = 2506,
    // Continue to add more Arweave type IDs here.
    // e.g.:
    // NextArweaveType = 2507,
    // AnotherArweaveType = 2508,
    // ...
    // LastArweaveType = 3499,

    // StarkNet types
    StarknetBlock = 3500,
    StarknetTransaction = 3501,
    StarknetTransactionTypeEnum = 3502,
    StarknetEvent = 3503,
    StarknetArrayBytes = 3504,
    // Continue to add more StarkNet type IDs here.
    // e.g.:
    // NextStarknetType = 3505,
    // AnotherStarknetType = 3506,
    // ...
    // LastStarknetType = 4499,

    // Reserved discriminant space for a future blockchain type IDs: [4,500, 5,499]
    //
    // Generated with the following shell script:
    //
    // ```
    // grep -Po "(?<=IndexForAscTypeId::)IDENDIFIER_PREFIX.*\b" SRC_FILE | sort |uniq | awk 'BEGIN{count=2500} {sub("$", " = "count",", $1); count++} 1'
    // ```
    //
    // INSTRUCTIONS:
    // 1. Replace the IDENTIFIER_PREFIX and the SRC_FILE placeholders according to the blockchain
    //    name and implementation before running this script.
    // 2. Replace `3500` part with the first number of that blockchain's reserved discriminant space.
    // 3. Insert the output right before the end of this block.
    UnitTestNetworkUnitTestTypeU32 = u32::MAX - 7,
    UnitTestNetworkUnitTestTypeU32Array = u32::MAX - 6,

    UnitTestNetworkUnitTestTypeU16 = u32::MAX - 5,
    UnitTestNetworkUnitTestTypeU16Array = u32::MAX - 4,

    UnitTestNetworkUnitTestTypeI8 = u32::MAX - 3,
    UnitTestNetworkUnitTestTypeI8Array = u32::MAX - 2,

    UnitTestNetworkUnitTestTypeBool = u32::MAX - 1,
    UnitTestNetworkUnitTestTypeBoolArray = u32::MAX,
}

pub trait ToAscObj<C: AscType> {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<C, AscError>;
}

impl ToAscObj<u32> for IndexForAscTypeId {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, _heap: &mut H) -> Result<u32, AscError> {
        Ok(*self as u32)
    }
}

pub fn padding_to_16(content_length: usize) -> usize {
    (16 - (HEADER_SIZE + content_length) % 16) % 16
}

pub fn asc_new<C, T: ?Sized, H: AscHeap + ?Sized>(
    heap: &mut H,
    rust_obj: &T,
) -> Result<AscPtr<C>, AscError>
where
    C: AscType + AscIndexId,
    T: ToAscObj<C>,
{
    let obj = rust_obj.to_asc_obj(heap)?;
    AscPtr::alloc_obj(obj, heap)
}

pub fn asc_get<T, C, H: AscHeap + ?Sized>(
    heap: &H,
    asc_ptr: AscPtr<C>,
    mut depth: usize,
) -> Result<T, AscError>
where
    C: AscType + AscIndexId,
    T: FromAscObj<C>,
{
    depth += 1;

    if depth > MAX_RECURSION_DEPTH {
        return Err(AscError::MaxRecursion);
    }

    T::from_asc_obj(asc_ptr.read_ptr(heap)?, heap, depth)
}

pub fn asc_get_optional<T, C, H: AscHeap + ?Sized>(
    heap: &H,
    asc_ptr: AscPtr<C>,
    depth: usize,
) -> Result<Option<T>, AscError>
where
    C: AscType + AscIndexId,
    T: FromAscObj<C>,
{
    if asc_ptr.is_null() {
        return Ok(None);
    }

    asc_get(heap, asc_ptr, depth).map(Some)
}

impl<C: AscType, T: ToAscObj<C>> ToAscObj<C> for &T {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<C, AscError> {
        (*self).to_asc_obj(heap)
    }
}

impl ToAscObj<bool> for bool {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, _heap: &mut H) -> Result<bool, AscError> {
        Ok(*self)
    }
}

pub trait FromAscObj<C: AscType>: Sized {
    fn from_asc_obj<H: AscHeap + ?Sized>(obj: C, heap: &H, depth: usize) -> Result<Self, AscError>;
}
