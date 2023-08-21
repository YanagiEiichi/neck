use tokio::io::{self, AsyncBufReadExt, AsyncRead};

use tokio::io::BufReader;

/// Read a group of lines ending with an empty line from a BufReader.
pub async fn read_lines<T: Unpin + AsyncRead>(
    stream: &mut BufReader<T>,
) -> io::Result<Vec<String>> {
    let mut lines: Vec<String> = Vec::new();
    let mut buf = String::new();
    loop {
        // The `buf` memory space is reused, so it must be cleared each time it is used.
        buf.clear();

        // Normally, the `read` method will wait for any bytes received, so zero bytes read indicate an EOF received.
        if stream.read_line(&mut buf).await? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Connection closed by peer",
            ));
        }

        // The `read_line` retains separator characters such as CR or LF at the end, which should be trimmed.
        let s = buf.trim_end();

        // If an empty line is received.
        if s.is_empty() {
            // And it is the first line of the current context, ignore it and continue reading the next line.
            // otherwise, finish reading and return read lines.
            if lines.is_empty() {
                continue;
            } else {
                break;
            }
        }

        // Now, it is not an empty line, create a copiable String and record it into `lines`.
        lines.push(String::from(s));
    }
    Ok(lines)
}
