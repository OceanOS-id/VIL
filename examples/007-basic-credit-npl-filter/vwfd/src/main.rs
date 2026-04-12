// 007 — NPL Account Flagger (Ruby Sidecar)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/007-basic-credit-npl-filter/vwfd/workflows", 3107)
        .sidecar("flag_npl_account", "ruby examples/007-basic-credit-npl-filter/vwfd/sidecar/ruby/npl_flag.rb")
        .run().await;
}
