//! 文件写入辅助。

use std::io::Write as _;
use std::path::Path;

/// 写文本文件。
///
/// `private = true` 时 Unix 下以 0600 权限创建/覆盖（ssh-keygen 同款行为）：
/// 私钥等敏感内容按默认 umask 会落成 0644，多用户机器上其他本地账号可读；
/// 覆盖已有宽权限文件时同样收紧到 0600。非 Unix 平台与 [`std::fs::write`] 一致。
pub fn write_text_file(path: &Path, content: &str, private: bool) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        file.write_all(content.as_bytes())?;
        if private {
            // OpenOptions::mode() 只对新建文件生效；覆盖已存在的宽权限文件时
            // 权限保持不变（残留 0644 的私钥任何本地账号可读），必须显式收紧
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = private;
        std::fs::write(path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn private_file_gets_restricted_permissions() {
        let dir = std::env::temp_dir().join("devtools-write-text-file-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("secret.key");

        // 断言不受进程 umask 影响：私钥的安全要求是 group/other 无权限
        //（显式 set_permissions(0600) 后恒成立）
        let group_other_bits = |p: &std::path::Path| {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::metadata(p).unwrap().permissions().mode() & 0o077
        };
        let owner_can_rw = |p: &std::path::Path| {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::metadata(p).unwrap().permissions().mode() & 0o600 == 0o600
        };

        write_text_file(&path, "content", true).unwrap();
        assert_eq!(group_other_bits(&path), 0);

        // 覆盖已有宽权限文件时收紧（mode() 对已存在文件不生效，实现须显式收紧）
        std::fs::write(&path, "old").unwrap();
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();
        write_text_file(&path, "new", true).unwrap();
        assert_eq!(group_other_bits(&path), 0);

        // 非敏感文件保持 owner 可读写（不做 group/other 严格断言）
        let plain = dir.join("plain.txt");
        write_text_file(&plain, "data", false).unwrap();
        assert!(owner_can_rw(&plain));

        std::fs::remove_dir_all(&dir).ok();
    }
}
