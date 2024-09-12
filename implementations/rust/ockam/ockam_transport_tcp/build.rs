use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        ebpf_alias: { all(target_os = "linux", feature = "ebpf") }
    }
}
