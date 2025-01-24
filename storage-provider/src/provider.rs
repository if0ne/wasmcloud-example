pub(crate) mod bindings {
    wit_bindgen_wrpc::generate!();
}

use std::sync::Arc;

use anyhow::Context as _;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::RwLock;
use tracing::info;
use wasmcloud_provider_sdk::initialize_observability;
use wasmcloud_provider_sdk::{
    run_provider, serve_provider_exports, Context, LinkConfig, LinkDeleteInfo, Provider,
    ProviderInitConfig,
};
use wgpu::RequestAdapterOptionsBase;
use wit_bindgen_wrpc::bytes::Bytes;

use crate::provider::bindings::exports::wasmcloud::example::fs_storage::Handler;

use crate::config::ProviderConfig;

#[derive(Clone)]
pub struct FsStorageProvider {
    config: Arc<RwLock<ProviderConfig>>,
    adapter: wgpu::Adapter,
}

impl FsStorageProvider {
    fn name() -> &'static str {
        "fs-storage-provider"
    }

    pub async fn run() -> anyhow::Result<()> {
        initialize_observability!(
            Self::name(),
            std::env::var_os("PROVIDER_CUSTOM_TEMPLATE_FLAMEGRAPH_PATH")
        );

        let adapter = {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

            instance.request_adapter(&RequestAdapterOptionsBase::default()).await.expect("Failed to fetch GPU")
        };

        let provider = FsStorageProvider {
            config: Default::default(),
            adapter,
        };
        
        let shutdown = run_provider(provider.clone(), FsStorageProvider::name())
            .await
            .context("failed to run provider")?;

        let connection = wasmcloud_provider_sdk::get_connection();
        serve_provider_exports(
            &connection
                .get_wrpc_client(connection.provider_key())
                .await?,
            provider,
            shutdown,
            bindings::serve,
        )
        .await
    }
}

impl Handler<Option<Context>> for FsStorageProvider {
    async fn store(
        &self,
        _cx: Option<Context>,
        filename: String,
        data: Bytes,
    ) -> anyhow::Result<Result<(), String>> {
        let filename = std::env::home_dir().unwrap_or_default().join(filename);

        info!("Create next file {:?}", filename);

        let dirs = filename.parent().unwrap();
        tokio::fs::create_dir_all(&dirs).await?;

        let file = match tokio::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&filename)
            .await
        {
            Ok(file) => file,
            Err(e) => {
                return Ok(Err(format!("Failed to open file: {}", e)));
            }
        };

        let footer = format!("\nGPU: {}\n", self.adapter.get_info().name);

        let mut writer = BufWriter::new(file);
        writer.write_all(&data).await?;
        writer.write_all(footer.as_bytes()).await?;
        writer.flush().await?;

        Ok(Ok(()))
    }

    async fn load(
        &self,
        _cx: Option<Context>,
        filename: String,
    ) -> anyhow::Result<Result<Bytes, String>> {
        let filename = std::env::home_dir().unwrap_or_default().join(filename);

        let file = match tokio::fs::OpenOptions::new()
            .read(true)
            .open(&filename)
            .await
        {
            Ok(file) => file,
            Err(e) => {
                return Ok(Err(format!("Failed to open file: {}", e)));
            }
        };

        let mut buf = vec![];

        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut buf).await?;
        reader.flush().await?;

        Ok(Ok(buf.into()))
    }
}

impl Provider for FsStorageProvider {
    async fn init(&self, config: impl ProviderInitConfig) -> anyhow::Result<()> {
        let provider_id = config.get_provider_id();
        let initial_config = config.get_config();
        info!(
            provider_id,
            ?initial_config,
            "initializing fs-storage provider"
        );

        *self.config.write().await = ProviderConfig::from(initial_config);

        Ok(())
    }
    async fn receive_link_config_as_source(&self, _link: LinkConfig<'_>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn receive_link_config_as_target(&self, _link: LinkConfig<'_>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn delete_link_as_source(&self, _link: impl LinkDeleteInfo) -> anyhow::Result<()> {
        Ok(())
    }

    async fn delete_link_as_target(&self, _link: impl LinkDeleteInfo) -> anyhow::Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
