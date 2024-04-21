// Only macros for testing, not for using in actual code
#[macro_export]
macro_rules! host_fn_test {
    ($datasource_name:expr, $guest_func:ident, $host:ident, $ptr:ident $body:block) => {
        #[::rstest::rstest]
        #[case("0.0.4")]
        #[case("0.0.5")]
        fn $guest_func(#[case] version: &str) {
            use convert_case::Case;
            use convert_case::Casing;
            use env_logger;
            use prometheus::default_registry;
            use std::env;
            use $crate::rpc_client::RpcAgent;

            env::set_var("SUBGRAPH_WASM_RUNTIME_TEST", "YES");

            env_logger::try_init().unwrap_or_default();

            let registry = default_registry();

            let (version, wasm_path) = get_subgraph_testing_resource(version, $datasource_name);

            let mut $host = mock_wasm_host(
                version,
                &wasm_path,
                registry,
                RpcAgent::new_mock(default_registry()),
                None
            );
            let wasm_test_func_name = format!("{}", stringify!($guest_func).to_case(Case::Camel));
            let func = $host
                .instance
                .exports
                .get_function(&wasm_test_func_name)
                .expect(&format!(
                    "No function with name `{wasm_test_func_name}` exists!",
                ));

            let result = func
                .call(&mut $host.store, &[])
                .expect("Calling function failed!");
            let $ptr = result.first().unwrap().unwrap_i32() as u32;

            $body
        }
    };

    ($datasource_name:expr, $guest_func:ident, $host:ident $body:block) => {
        #[::rstest::rstest]
        #[case("0.0.4")]
        #[case("0.0.5")]
        fn $guest_func(#[case] version: &str) {
            use convert_case::Case;
            use convert_case::Casing;
            use env_logger;
            use std::env;
            use $crate::rpc_client::RpcAgent;

            use prometheus::default_registry;
            let registry = default_registry();

            env::set_var("SUBGRAPH_WASM_RUNTIME_TEST", "YES");
            env_logger::try_init().unwrap_or_default();
            let (version, wasm_path) = get_subgraph_testing_resource(version, $datasource_name);

            let mut $host =
                mock_wasm_host(version, &wasm_path, registry, RpcAgent::new_mock(registry), None);
            let wasm_test_func_name = format!("{}", stringify!($guest_func).to_case(Case::Camel));
            let func = $host
                .instance
                .exports
                .get_function(&wasm_test_func_name)
                .expect(&format!(
                    "No function with name `{wasm_test_func_name}` exists!",
                ));

            let result = func
                .call(&mut $host.store, &[])
                .expect("Calling function failed!");
            assert!(result.is_empty());
            $body
        }
    };

    ($datasource_name:expr, $guest_func:ident, $host:ident, $result:ident $construct_args:block $handle_result:block) => {
        #[::rstest::rstest]
        #[case("0.0.4")]
        #[case("0.0.5")]
        fn $guest_func(#[case] version: &str) {
            use convert_case::Case;
            use convert_case::Casing;
            use env_logger;
            use std::env;
            use $crate::rpc_client::RpcAgent;

            env::set_var("SUBGRAPH_WASM_RUNTIME_TEST", "YES");
            env_logger::try_init().unwrap_or_default();

            use prometheus::default_registry;
            let registry = default_registry();

            let (version, wasm_path) = get_subgraph_testing_resource(version, $datasource_name);
            let mut $host =
                mock_wasm_host(version, &wasm_path, registry, RpcAgent::new_mock(registry), None);

            let args = $construct_args;

            let wasm_test_func_name = format!("{}", stringify!($guest_func).to_case(Case::Camel));
            let func = $host
                .instance
                .exports
                .get_function(&wasm_test_func_name)
                .expect(&format!(
                    "No function with name `{wasm_test_func_name}` exists!",
                ));

            let $result = func
                .call(&mut $host.store, &args)
                .expect("Calling function failed!");

            $handle_result
        }
    };
}
