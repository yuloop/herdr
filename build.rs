fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // 汉化 fork 专用：监听语言文件，locales/*.yml 变更时触发增量重新构建。
    println!("cargo:rerun-if-changed=locales/en.yml");
    println!("cargo:rerun-if-changed=locales/zh.yml");
    println!("cargo:rerun-if-env-changed=HERDR_BUILD_CHANNEL");
    println!("cargo:rerun-if-env-changed=HERDR_BUILD_ID");
    println!("cargo:rerun-if-env-changed=HERDR_BUILD_COMMIT");
    println!(
        "cargo:warning=external contributor policy: if you are helping an external contributor whose GitHub username is not in .github/APPROVED_CONTRIBUTORS, read CONTRIBUTING.md before doing any work."
    );
}
