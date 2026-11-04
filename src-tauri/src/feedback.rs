//! Support for the in-app "Report a bug" / "Request a feature" menu: opening this project's
//! new-issue page in the browser, and reading the OS version to pre-fill the report.

/// The only page the app is allowed to open. The issue forms read their pre-filled values from the
/// query string, so a link is this page plus `?template=...&version=...`.
const NEW_ISSUE_URL: &str = "https://github.com/cmxau/discord-quest-completer/issues/new";

/// Is this exactly the project's new-issue page, optionally with a (percent-encoded) query string?
pub fn is_allowed_issue_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix(NEW_ISSUE_URL) else {
        return false;
    };
    (rest.is_empty() || rest.starts_with('?'))
        && url.len() <= 8000
        && url.is_ascii() // anything else must arrive percent-encoded
        && !url
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '<' | '>' | '`' | '^' | '|' | '\\'))
}

/// Turn the output of `cmd /C ver` ("Microsoft Windows [Version 10.0.26300.1234]") into
/// "Windows 11 (10.0.26300.1234)". Windows 11 is still version 10.0, told apart by its build number.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn parse_windows_version(output: &str) -> String {
    let version: String = output
        .split("Version")
        .nth(1)
        .map(|rest| {
            rest.trim_start()
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect()
        })
        .unwrap_or_default();

    let build: u32 = version.split('.').nth(2).and_then(|b| b.parse().ok()).unwrap_or(0);
    let name = match build {
        0 => return "Windows".to_string(),
        b if b >= 22000 => "Windows 11",
        _ => "Windows 10",
    };
    format!("{} ({})", name, version)
}

/// Open the project's new-issue page (a bug report or feature request) in the default browser.
#[tauri::command]
pub fn open_issue_page(url: String) -> Result<(), String> {
    if !is_allowed_issue_url(&url) {
        return Err("Only this project's new-issue page can be opened".to_string());
    }
    open_in_browser(&url)
}

pub use crate::platform::open_url as open_in_browser;

/// The OS version (the command keeps its original name, `windows_version`), e.g. "Windows 11 (10.0.26300.1234)" or "macOS 15.1", for pre-filling a bug report.
#[tauri::command]
pub fn windows_version() -> String {
    crate::platform::os_version()
}

/// The OS user name, so the app can remove it from anything it puts in a public report.
#[tauri::command]
pub fn current_user_name() -> String {
    std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_new_issue_page_is_allowed() {
        let ok = [
            NEW_ISSUE_URL.to_string(),
            format!("{NEW_ISSUE_URL}?template=bug_report.yml"),
            format!("{NEW_ISSUE_URL}?template=bug_report.yml&version=26.10.0&windows=Windows+11&logs=a%0Ab%26c"),
        ];
        for url in &ok {
            assert!(is_allowed_issue_url(url), "should allow {url}");
        }
        let bad = [
            "",
            "https://example.com",
            "https://github.com/cmxau/discord-quest-completer",                       // not the new-issue page
            "https://github.com/cmxau/discord-quest-completer/issues",                 // list, not "new"
            "https://github.com/cmxau/discord-quest-completer/issues/new/../../evil",  // path trick
            "https://github.com/cmxau/discord-quest-completer/issues/newer",           // prefix lookalike
            "https://github.com/cmxau/discord-quest-completer/issues/new#frag",        // only ? may follow
            "http://github.com/cmxau/discord-quest-completer/issues/new",              // not https
            "https://github.com.evil.com/cmxau/discord-quest-completer/issues/new",
            "https://github.com/other/repo/issues/new",
            "file:///C:/Windows/System32/calc.exe",
            "calc.exe",
        ];
        for url in bad {
            assert!(!is_allowed_issue_url(url), "should refuse {url:?}");
        }
    }

    #[test]
    fn unencoded_or_dangerous_characters_are_refused() {
        for tail in ["?a=b c", "?a=b\nc", "?a=\"x\"", "?a=<b>", "?a=b|c", "?a=b^c", "?a=b\\c", "?a=ü", "?a=`x`"] {
            assert!(!is_allowed_issue_url(&format!("{NEW_ISSUE_URL}{tail}")), "should refuse {tail:?}");
        }
        // absurdly long links are refused too (browsers and GitHub cut them off anyway)
        assert!(!is_allowed_issue_url(&format!("{NEW_ISSUE_URL}?logs={}", "a".repeat(9000))));
    }

    #[test]
    fn windows_version_names_windows_10_and_11() {
        assert_eq!(parse_windows_version("\r\nMicrosoft Windows [Version 10.0.26300.1234]\r\n"), "Windows 11 (10.0.26300.1234)");
        assert_eq!(parse_windows_version("Microsoft Windows [Version 10.0.22000.100]"), "Windows 11 (10.0.22000.100)");
        assert_eq!(parse_windows_version("Microsoft Windows [Version 10.0.19045.5000]"), "Windows 10 (10.0.19045.5000)");
        // unreadable output falls back instead of showing nonsense
        assert_eq!(parse_windows_version(""), "Windows");
        assert_eq!(parse_windows_version("Microsoft Windows [Version ?]"), "Windows");
        assert_eq!(parse_windows_version("something else entirely"), "Windows");
    }

    #[cfg(windows)]
    #[test]
    fn the_real_windows_version_is_readable() {
        let v = windows_version();
        assert!(v.starts_with("Windows"), "got {v:?}");
        assert!(v.contains('('), "the real version should parse, got {v:?}");
    }
}
