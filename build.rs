fn main() {
    #[cfg(feature = "mini-dashboard")]
    mini_dashboard::setup_mini_dashboard().expect("Could not load the mini-dashboard assets");
}

#[cfg(feature = "mini-dashboard")]
mod mini_dashboard {
    use std::env;
    use std::fs::{self, create_dir_all, File};
    use std::io::{Read, Write};
    use std::path::PathBuf;

    use anyhow::{anyhow, Context};
    use cargo_toml::Manifest;
    use static_files::resource_dir;
    use zip::read::ZipArchive;

    pub fn setup_mini_dashboard() -> anyhow::Result<()> {
        let cargo_manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let cargo_toml = cargo_manifest_dir.join("Cargo.toml");
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        let sha1_path = out_dir.join(".mini-dashboard.sha1");
        let dashboard_dir = out_dir.join("mini-dashboard");

        let manifest = Manifest::from_path(&cargo_toml)
            .map_err(|e| anyhow!("Failed to parse Cargo.toml: {}", e))?;

        let package = manifest.package.ok_or_else(|| anyhow!("package not specified in Cargo.toml"))?;
        let metadata = package.metadata.as_ref().ok_or_else(|| anyhow!("no metadata specified in Cargo.toml"))?;
        let meta = &metadata["mini-dashboard"];

        // === 新增：检查是否使用本地文件 ===
        if let Ok(local_path) = env::var("MEILI_LOCAL_DASHBOARD_PATH") {
            println!("Using local dashboard from: {}", local_path);
            
            // 确保目录存在
            create_dir_all(&dashboard_dir)?;
            
            // 解压本地文件
            let file = File::open(&local_path)
                .with_context(|| format!("Failed to open local dashboard file: {}", local_path))?;
            let mut zip = ZipArchive::new(file)?;
            zip.extract(&dashboard_dir)?;
            
            // 嵌入资源
            resource_dir(&dashboard_dir).build()?;
            
            // 创建假的SHA1文件避免重复处理
            File::create(&sha1_path)?.write_all(b"local")?;
            
            return Ok(());
        }
        // === 结束新增 ===

        // 检查是否已有现成的dashboard
        if sha1_path.exists() && dashboard_dir.exists() {
            let mut sha1_file = File::open(&sha1_path)?;
            let mut sha1 = String::new();
            sha1_file.read_to_string(&mut sha1)?;
            if sha1 == meta["sha1"].as_str().unwrap() {
                return Ok(());
            }
        }

        // === 修改：允许file协议 ===
        let url = meta["assets-url"].as_str().unwrap();
        let dashboard_assets_bytes = if url.starts_with("file://") {
            // 处理本地文件
            let path = url.trim_start_matches("file://");
            let mut file = File::open(path)
                .with_context(|| format!("Failed to open local file: {}", path))?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            bytes
        } else {
            // 原始的网络下载逻辑（保留但可选）
            // 注意：这里移除了reqwest依赖，因为你可能没有网络
            // 实际使用时可以保留或移除
            unimplemented!("Network download disabled. Use MEILI_LOCAL_DASHBOARD_PATH")
        };

        // === 修改：跳过SHA1校验 ===
        // 直接解压文件而不校验
        create_dir_all(&dashboard_dir)?;
        let cursor = std::io::Cursor::new(dashboard_assets_bytes);
        let mut zip = ZipArchive::new(cursor)?;
        zip.extract(&dashboard_dir)?;
        resource_dir(&dashboard_dir).build()?;

        // 写入假SHA1值避免下次处理
        File::create(&sha1_path)?.write_all(b"skipped")?;

        Ok(())
    }
}
