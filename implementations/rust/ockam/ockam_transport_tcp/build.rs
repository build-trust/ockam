use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        ebpf_alias: { all(target_os = "linux", feature = "ebpf") }
    }

    #[cfg(all(target_os = "linux", feature = "ebpf"))]
    {
        use std::env;
        use std::path::PathBuf;
        use std::str::FromStr;
        use std::time::Duration;
        use url::Url;

        let out_dir = env::var("OUT_DIR").unwrap();

        let output_file = PathBuf::from_str(&out_dir).unwrap().join("ockam_ebpf");

        // TODO: Handle updates
        if output_file.exists() {
            return;
        }

        let url = "https://github.com/SanjoDeundiak/ebpf-test/releases/download/0.1.0/ockam_ebpf";

        let url = Url::parse(url).unwrap().join("ockam_ebpf").unwrap();

        let client_builder = reqwest::blocking::Client::builder();

        // TODO: Also respect other variables, like CARGO_HTTP_PROXY
        let client_builder = if let Ok(http_timeout) = env::var("CARGO_HTTP_TIMEOUT") {
            if let Ok(http_timeout) = u64::from_str(&http_timeout) {
                client_builder.timeout(Some(Duration::from_secs(http_timeout)))
            } else {
                client_builder
            }
        } else {
            client_builder
        };

        let client = client_builder.build().unwrap();

        let ebpf = client
            .get(url)
            .send()
            .expect("Error downloading eBPF")
            .bytes()
            .expect("Error downloading eBPF");

        std::fs::write(&output_file, ebpf).unwrap();
    }
}
