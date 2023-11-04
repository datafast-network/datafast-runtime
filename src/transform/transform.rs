use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::FromAscObj;
use crate::chain::ethereum::block::AscEthereumBlock;
use crate::chain::ethereum::block::EthereumFullBlock;
use crate::chain::ethereum::log::AscLogArray;
use crate::chain::ethereum::transaction::AscTransactionArray;
use crate::config::Config;
use crate::config::TransformConfig;
use crate::transform::errors::TransformError;
use crate::wasm_host::AscHost;
use std::collections::HashMap;
use wasmer::Function;
use wasmer::Value;

#[derive(Clone, Debug)]
pub struct TransformRequest {
    value: serde_json::Value,
    transform: TransformConfig,
}

pub struct TransformFunction {
    name: String,
    func: Function,
}

pub struct Transform {
    host: AscHost,
    funcs: HashMap<String, TransformFunction>,
}

impl Transform {
    pub fn new(host: AscHost, conf: &Config) -> Result<Self, TransformError> {
        let mut funcs = HashMap::new();
        assert!(conf.transforms.is_some());
        let transforms = conf.transforms.as_ref().unwrap();
        for (name, transform) in transforms {
            let func = host
                .instance
                .exports
                .get_function(&transform.func_name)?
                .to_owned();
            funcs.insert(
                name.clone(),
                TransformFunction {
                    name: transform.func_name.clone(),
                    func,
                },
            );
        }
        Ok(Transform { host, funcs })
    }

    pub fn transform_data<P: AscType + AscIndexId, R: FromAscObj<P>>(
        &mut self,
        request: &TransformRequest,
    ) -> Result<R, TransformError> {
        let func_name = request.transform.func_name.clone();
        let func = self
            .funcs
            .get(&func_name)
            .ok_or(TransformError::InvalidFunctionName(func_name))?;

        let mut json_data = request.value.clone();
        let asc_json = asc_new(&mut self.host, &mut json_data)?;
        let ptr = asc_json.wasm_ptr();
        let result = func
            .func
            .call(&mut self.host.store, &[Value::I32(ptr as i32)])?;

        let asc_ptr = AscPtr::<P>::new(result.first().unwrap().unwrap_i32() as u32);
        let result = asc_get(&self.host, asc_ptr, 0).expect("Failed to get result");
        Ok(result)
    }

    /// ## Transform full block for delta Ingestor only
    /// Require:
    /// - block header request `transformEthereumBlock`
    /// - block transactions request `transformEthereumTxs`
    /// - block logs request `transformEthereumLogs`
    pub fn transform_block(
        &mut self,
        block_requests: Vec<TransformRequest>,
    ) -> Result<EthereumFullBlock, TransformError> {
        let request_header = block_requests
            .iter()
            .find(|r| r.transform.func_name == "transformEthereumBlock")
            .expect("transformEthereumBlock notfound");
        let header = self.transform_data::<AscEthereumBlock, _>(request_header)?;
        let request_tx = block_requests
            .iter()
            .find(|r| r.transform.func_name == "transformEthereumTxs")
            .expect("transformEthereumTxs notfound");
        let transactions = self.transform_data::<AscTransactionArray, _>(request_tx)?;
        let request_logs = block_requests
            .iter()
            .find(|r| r.transform.func_name == "transformEthereumLogs")
            .expect("transformEthereumLogs notfound");
        let logs = self.transform_data::<AscLogArray, _>(request_logs)?;
        let full_block = EthereumFullBlock::from((header, transactions, logs));
        Ok(full_block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ethereum::block::AscEthereumBlock;
    use crate::chain::ethereum::log::AscLogArray;
    use crate::chain::ethereum::transaction::AscTransactionArray;
    use crate::messages::SubgraphData;
    use crate::wasm_host::test::get_subgraph_testing_resource;
    use crate::wasm_host::test::mock_wasm_host;
    use std::fs::File;
    use tokio::join;

    #[tokio::test]
    async fn test_transform_block() {
        env_logger::try_init().unwrap_or_default();
        let transform_block = TransformConfig {
            datasource: "TestTypes".to_string(),
            func_name: "transformEthereumBlock".to_string(),
        };
        let mut transforms = HashMap::new();
        transforms.insert(transform_block.func_name.clone(), transform_block.clone());
        let conf = Config {
            subgraph_name: "".to_string(),
            subgraph_id: None,
            manifest: "".to_string(),
            transforms: Some(transforms),
        };
        let (version, wasm_path) =
            get_subgraph_testing_resource("0.0.5", &transform_block.datasource);
        let host = mock_wasm_host(version, &wasm_path);
        let mut transform = Transform::new(host, &conf).unwrap();
        let (s1, r1) = kanal::bounded_async(1);
        let (s2, r2) = kanal::bounded_async(1);

        let t1 = async move {
            while let Ok(request) = r1.recv().await {
                let result = transform
                    .transform_data::<AscEthereumBlock, _>(&request)
                    .unwrap();
                s2.send(SubgraphData::Block(result)).await.unwrap();
                return;
            }
        };

        // Collecting result from transformer
        let t2 = async move {
            while let Ok(SubgraphData::Block(block)) = r2.recv().await {
                ::log::info!("Transformed data: \n{:?}\n", block);
                assert_eq!(block.number.to_string(), "10000000");
                assert_eq!(
                    format!("{:?}", block.hash),
                    "0xaa20f7bde5be60603f11a45fc4923aab7552be775403fc00c2e6b805e6297dbe"
                );
                return;
            }
            panic!("test failed");
        };

        let start = std::time::Instant::now();
        // let file_json = File::open("/Users/quannguyen/block_10000000_safe_size.json").unwrap();
        let file_json = File::open("./block.json").unwrap();
        // Send test data for transform
        let ingestor_block = serde_json::from_reader(file_json).unwrap();
        ::log::info!("Input data success {:?}", start.elapsed());
        // log::info!("block: {:?}", ingestor_block);
        let request = TransformRequest {
            value: ingestor_block,
            transform: transform_block.clone(),
        };

        // Collecting the threads
        let _result = join!(t1, t2, s1.send(request));
    }

    #[tokio::test]
    async fn test_transform_txs() {
        env_logger::try_init().unwrap_or_default();
        let transform_block = TransformConfig {
            datasource: "TestTypes".to_string(),
            func_name: "transformEthereumTxs".to_string(),
        };
        let mut transforms = HashMap::new();
        transforms.insert(transform_block.func_name.clone(), transform_block.clone());
        let conf = Config {
            subgraph_name: "".to_string(),
            subgraph_id: None,
            manifest: "".to_string(),
            transforms: Some(transforms),
        };
        let (version, wasm_path) =
            get_subgraph_testing_resource("0.0.5", &transform_block.datasource);
        let host = mock_wasm_host(version, &wasm_path);
        let mut transform = Transform::new(host, &conf).unwrap();
        let (s1, r1) = kanal::bounded_async(1);
        let (s2, r2) = kanal::bounded_async(1);

        let t1 = async move {
            while let Ok(request) = r1.recv().await {
                let result = transform
                    .transform_data::<AscTransactionArray, _>(&request)
                    .unwrap();
                s2.send(SubgraphData::Transactions(result)).await.unwrap();
                return;
            }
        };

        // Collecting result from transformer
        let t2 = async move {
            while let Ok(SubgraphData::Transactions(txs)) = r2.recv().await {
                assert_eq!(txs.len(), 2);
                let tx = txs.first().unwrap();
                assert_eq!(
                    format!("{:?}", tx.hash),
                    "0x4a1e3e3a2aa4aa79a777d0ae3e2c3a6de158226134123f6c14334964c6ec70cf"
                );
                assert_eq!(tx.index.to_string(), "0");
                assert!(tx.to.is_some());
                assert_eq!(
                    format!("{:?}", tx.to.unwrap()),
                    "0x60f18d941f6253e3f7082ea0db3bc3944e7e9d40"
                );
                assert_eq!(
                    format!("{:?}", tx.from),
                    "0xea674fdde714fd979de3edf0f56aa9716b898ec8"
                );
                assert_eq!(format!("{:?}", tx.value), "1037716102333920200321");
                assert_eq!(format!("{:?}", tx.gas_limit), "0");
                assert_eq!(format!("{:?}", tx.gas_price), "68719476736");
                return;
            }
            panic!("test failed");
        };

        let start = std::time::Instant::now();
        let file_json = File::open("./block.json").unwrap();
        // Send test data for transform
        let ingestor_block: serde_json::Value = serde_json::from_reader(file_json).unwrap();
        ::log::info!("Input data success {:?}", start.elapsed());
        let txs: serde_json::Value = ingestor_block.get("transactions").unwrap().clone();
        let request = TransformRequest {
            value: txs,
            transform: transform_block.clone(),
        };

        // Collecting the threads
        let _result = join!(t1, t2, s1.send(request));
    }

    #[tokio::test]
    async fn test_transform_logs() {
        env_logger::try_init().unwrap_or_default();
        let transform_block = TransformConfig {
            datasource: "TestTypes".to_string(),
            func_name: "transformEthereumLogs".to_string(),
        };
        let mut transforms = HashMap::new();
        transforms.insert(transform_block.func_name.clone(), transform_block.clone());
        let conf = Config {
            subgraph_name: "".to_string(),
            subgraph_id: None,
            manifest: "".to_string(),
            transforms: Some(transforms),
        };
        let (version, wasm_path) =
            get_subgraph_testing_resource("0.0.5", &transform_block.datasource);
        let host = mock_wasm_host(version, &wasm_path);
        let mut transform = Transform::new(host, &conf).unwrap();
        let (s1, r1) = kanal::bounded_async(1);
        let (s2, r2) = kanal::bounded_async(1);

        let t1 = async move {
            while let Ok(request) = r1.recv().await {
                let result = transform
                    .transform_data::<AscLogArray, _>(&request)
                    .unwrap();
                s2.send(SubgraphData::Logs(result)).await.unwrap();
                return;
            }
        };

        // Collecting result from transformer
        let t2 = async move {
            while let Ok(SubgraphData::Logs(logs)) = r2.recv().await {
                assert_eq!(logs.len(), 2);
                let log = logs.first().unwrap();
                assert_eq!(
                    format!("{:?}", log.address),
                    "0xced4e93198734ddaff8492d525bd258d49eb388e"
                );
                assert!(log.log_index.is_some());
                assert_eq!(format!("{:?}", log.log_index.unwrap()), "0");
                assert_eq!(format!("{:?}", log.block_number.unwrap()), "10000000");
                return;
            }
            panic!("test failed");
        };

        let start = std::time::Instant::now();
        let file_json = File::open("./block.json").unwrap();
        // Send test data for transform
        let ingestor_block: serde_json::Value = serde_json::from_reader(file_json).unwrap();
        ::log::info!("Input data success {:?}", start.elapsed());
        let txs: serde_json::Value = ingestor_block.get("logs").unwrap().clone();
        let request = TransformRequest {
            value: txs,
            transform: transform_block.clone(),
        };

        // Collecting the threads
        let _result = join!(t1, t2, s1.send(request));
    }

    #[tokio::test]
    async fn test_transform_full_block() {
        env_logger::try_init().unwrap_or_default();
        let mut transforms = HashMap::new();
        for i in vec![
            "transformEthereumBlock",
            "transformEthereumLogs",
            "transformEthereumTxs",
        ] {
            let transform_block = TransformConfig {
                datasource: "TestTypes".to_string(),
                func_name: i.to_string(),
            };
            transforms.insert(transform_block.func_name.clone(), transform_block.clone());
        }
        let conf = Config {
            subgraph_name: "".to_string(),
            subgraph_id: None,
            manifest: "".to_string(),
            transforms: Some(transforms),
        };
        let (version, wasm_path) = get_subgraph_testing_resource("0.0.5", "TestTypes");
        let host = mock_wasm_host(version, &wasm_path);
        let mut transform = Transform::new(host, &conf).unwrap();
        let file_json = File::open("./block.json").unwrap();
        // Send test data for transform
        let ingestor_block: serde_json::Value = serde_json::from_reader(file_json).unwrap();
        let requests = vec![
            TransformRequest {
                value: ingestor_block.clone(),
                transform: TransformConfig {
                    datasource: "TestTypes".to_string(),
                    func_name: "transformEthereumBlock".to_string(),
                },
            },
            TransformRequest {
                value: ingestor_block.get("transactions").unwrap().clone(),
                transform: TransformConfig {
                    datasource: "TestTypes".to_string(),
                    func_name: "transformEthereumTxs".to_string(),
                },
            },
            TransformRequest {
                value: ingestor_block.get("logs").unwrap().clone(),
                transform: TransformConfig {
                    datasource: "TestTypes".to_string(),
                    func_name: "transformEthereumLogs".to_string(),
                },
            },
        ];
        let block = transform.transform_block(requests).unwrap();
        log::info!("{:?}", block);
        assert_eq!(format!("{:?}", block.header.gas_limit), "9990236");
        assert_eq!(format!("{:?}", block.number), "10000000");
        //asert_eq all fields of block
        assert_eq!(block.transactions.len(), 2);
        assert_eq!(block.logs.len(), 2);
    }
}
