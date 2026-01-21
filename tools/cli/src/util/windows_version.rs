use anyhow::{Result, bail};
use semver::Version;

pub fn normalize_windows_version(version: &str) -> Result<String> {
  let parsed =
    Version::parse(version).map_err(|_| anyhow::anyhow!("invalid app version: '{version}'"))?;
  if !parsed.build.is_empty() {
    bail!("invalid app version: '{version}'");
  }

  if parsed.major > 255 || parsed.minor > 255 {
    bail!(
      "invalid app version: '{version}', major and minor version must be less than 256 for \
       windows msi."
    );
  }

  let revision = if parsed.pre.is_empty() {
    65535
  } else {
    let pre = parsed.pre.as_str().to_lowercase();
    let digits: String = pre
      .chars()
      .filter(|c| c.is_ascii_digit())
      .collect();
    let pre_num = digits.parse::<u32>().unwrap_or(0);
    let weight = if pre.contains("alpha") {
      0
    } else if pre.contains("beta") {
      20000
    } else if pre.contains("rc") {
      40000
    } else {
      0
    };
    weight + (pre_num % 20000)
  };

  if parsed.patch > 65535 {
    bail!(
      "invalid app version: '{version}', patch version must be less than 65536 for windows msi."
    );
  }

  // Return a clean 4-segment numeric version for MSI ProductVersion.
  Ok(format!("{}.{}.{}.{}", parsed.major, parsed.minor, parsed.patch, revision))
}
