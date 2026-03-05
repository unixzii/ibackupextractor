pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let size = bytes as f64;

    if size < KB {
        format!("{bytes} B")
    } else if size < MB {
        format!("{:.2} KiB", size / KB)
    } else if size < GB {
        format!("{:.2} MiB", size / MB)
    } else {
        format!("{:.2} GiB", size / GB)
    }
}
