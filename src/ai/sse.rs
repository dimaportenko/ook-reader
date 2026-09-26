#[derive(Debug, Default)]
pub(crate) struct SseBuffer {
    pending: Vec<u8>,
}

impl SseBuffer {
    pub(crate) fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.pending.extend_from_slice(chunk);

        let mut payloads = Vec::new();
        while let Some(end) = self.pending.iter().position(|&byte| byte == b'\n') {
            let bytes: Vec<u8> = self.pending.drain(..=end).collect();
            let text = String::from_utf8_lossy(&bytes);
            let line = text.trim_end_matches(['\n', '\r']);

            if let Some(data) = line.strip_prefix("data:") {
                payloads.push(data.strip_prefix(' ').unwrap_or(data).to_owned());
            }
        }

        payloads
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn a_whole_event_yields_its_payload() {
        let mut sse = SseBuffer::default();

        assert_eq!(sse.push(b"data: {\"a\":1}\n\n"), ["{\"a\":1}"]);
    }

    #[test]
    fn a_line_split_across_chunks_waits_for_the_rest() {
        let mut sse = SseBuffer::default();

        assert!(
            sse.push(b"data: {\"a\"").is_empty(),
            "no newline yet, no line"
        );
        assert_eq!(sse.push(b":1}\n\n"), ["{\"a\":1}"]);
    }

    #[test]
    fn two_events_in_one_chunk_yield_both() {
        let mut sse = SseBuffer::default();

        assert_eq!(sse.push(b"data: one\n\ndata: two\n\n"), ["one", "two"]);
    }

    #[test]
    fn crlf_line_endings_are_stripped() {
        let mut sse = SseBuffer::default();

        assert_eq!(sse.push(b"data: one\r\n\r\n"), ["one"]);
    }

    #[test]
    fn a_character_split_across_chunks_survives() {
        let mut sse = SseBuffer::default();

        assert!(sse.push(b"data: caf\xC3").is_empty());
        assert_eq!(
            sse.push(b"\xA9\n\n"),
            ["caf\u{e9}"],
            "the two bytes of \u{e9} arrived in different chunks"
        );
    }

    #[test]
    fn lines_that_are_not_data_are_skipped() {
        let mut sse = SseBuffer::default();

        assert_eq!(
            sse.push(b": keep-alive\nevent: message\n\ndata: one\n\n"),
            ["one"]
        );
    }
}
