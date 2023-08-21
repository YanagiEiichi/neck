use tokio::io::{self, AsyncBufReadExt, AsyncRead, Error, ErrorKind};

use tokio::io::BufReader;

/// Read a group of lines ending with an empty line from the stream.
/// NOTE: Empty lines at the beginning will be ignored.
pub async fn read_lines<T: Unpin + AsyncRead>(
    stream: &mut BufReader<T>,
    budget: &mut usize,
) -> io::Result<Vec<String>> {
    let mut lines: Vec<String> = Vec::new();
    loop {
        // Read a line.
        let line = read_line(stream, budget).await?;

        // If an empty line is received.
        if line.is_empty() {
            // And it is the first line of the current context, ignore it and continue reading the next line.
            // otherwise, finish reading and return read lines.
            if lines.is_empty() {
                continue;
            } else {
                break;
            }
        }

        // Now, it is not an empty line, create a copiable String and record it into `lines`.
        lines.push(line);
    }
    Ok(lines)
}

/// Read a line from the stream.
/// NOTE: The CRLF characters will not be included in the result.
pub async fn read_line<T: AsyncRead + Unpin>(
    stream: &mut BufReader<T>,
    budget: &mut usize,
) -> Result<String, Error> {
    // Read until '\n'.
    let mut sb = read_until(stream, '\n', budget).await?;

    // Remove the trailing '\r', if it is present.
    // Since the HTTP header separator is CRLF, and the `read_until` function only removes the LF character,
    // so the CR character should be removed at this point.
    if sb.ends_with('\r') {
        sb.pop();
    }

    Ok(sb)
}

/// Read the stream until the terminal character is encountered.
/// NOTE: The terminal character will not be included in the result.
pub async fn read_until<T: Unpin + AsyncRead>(
    stream: &mut BufReader<T>,
    terminal: char,
    budget: &mut usize,
) -> Result<String, Error> {
    let mut buffer = String::new();
    let mut done = false;

    while !done {
        // The `fill_buf` method will wait for any bytes received, so zero bytes read indicate an EOF received.
        let tmp = stream.fill_buf().await?;
        if tmp.len() == 0 {
            Err(Error::new(
                ErrorKind::BrokenPipe,
                "Connection closed by peer",
            ))?;
        }

        let mut tmp_index = 0;
        for c in tmp {
            tmp_index += 1;

            // Consuming and checking the budget, and returning an error message if it is zero.
            *budget -= 1;
            if *budget <= 0 {
                Err(Error::new(ErrorKind::OutOfMemory, "Limit overlfow"))?;
            }

            // If the current character is a terminal character, break this loop.
            // Otherwise, save the current character to the `buf`.
            if *c == terminal as u8 {
                done = true;
                break;
            } else {
                buffer.push(*c as char);
            }
        }

        stream.consume(tmp_index);
    }

    Ok(buffer)
}
