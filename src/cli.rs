pub fn validate_time(s: &str) -> Result<u64, String> {
    let time: u64 = s
        .parse()
        .map_err(|_| format!("`{}` is not a valid number", s))?;
    if time == 0 || time > 1440 {
        Err("Time must be between 1 and 1440 minutes".to_string())
    } else {
        Ok(time)
    }
}
