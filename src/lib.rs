use std::str::CharIndices;

const ESC: char = '\x1b';
// const LF: char = '\n';
// const CR: char = '\r';
const CSI: char = '[';
const OSC: char = ']';
const DCS: char = 'P';
const SOC: char = 'X';
const PM: char = '^';
const APC: char = '_';
const BEL: char = '\x07';
const ST_CHAR: char = '\\';
#[allow(dead_code)]
const ST: &str = "\x1b\\";

/// Trait to strip out ANSI Escape sequences.
pub trait NonEsc<'a> {
    fn non_esc(self) -> AnsiStripper<'a>;
}

/// Implement the trait for string slices.
impl<'a> NonEsc<'a> for &'a str {
    fn non_esc(self) -> AnsiStripper<'a> {
        AnsiStripper::new(self)
    }
}

/// Current mode of the iterator.
#[derive(PartialEq, Debug)]
enum Mode {
    Normal,
    InEsc,
    AwaitSt,
    InOsc,
    InCsi,
    OscMaybeSt,
    MaybeSt,
}

/// At each iteration, returns the next substring that doesn't contain an ANSI escape code.
pub struct AnsiStripper<'a> {
    src: &'a str,
    char_indices: CharIndices<'a>,
    prev_index: usize,
    prev_char: Option<char>,
}

/// Create an AnsiStripper against a string slice.
impl<'a> AnsiStripper<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            char_indices: src.char_indices(),
            prev_index: 0,
            prev_char: None,
        }
    }
}

impl<'a> Iterator for AnsiStripper<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let mut curr_index: usize;
        let mut curr_char: char;

        // Get the first char (and index) to work with in this iteration.
        // If there was (index, Option<char>) left over from last iteration, use that.
        if self.prev_char.is_some() {
            // there were leftovers from last iteration...
            (curr_index, curr_char) = (self.prev_index, self.prev_char.unwrap());
            self.prev_char = None;
        } else {
            // There were no leftovers to consider, so pull the next char...
            (curr_index, curr_char) = match self.char_indices.next() {
                Some((i, c)) => (i, c),
                // No leftovers and nothing left: just exit.
                None => return None,
            }
        };

        let mut start_index = curr_index;
        let mut end_index = curr_index + curr_char.len_utf8();
        let mut mode = if curr_char == ESC {
            Mode::InEsc
        } else {
            Mode::Normal
        };

        loop {
            // Test and assign the next character.
            (curr_index, curr_char) = match self.char_indices.next() {
                Some((i, c)) => (i, c),
                None => {
                    return match (mode, end_index > start_index) {
                        (Mode::Normal, true) => Some(&self.src[start_index..end_index]),
                        _ => None,
                    }
                }
            };

            end_index = curr_index + curr_char.len_utf8();

            match mode {
                Mode::Normal => {
                    if curr_char == ESC {
                        // We're moving from Normal to InEsc...
                        self.prev_index = curr_index;
                        self.prev_char = Some(curr_char);
                        if curr_index > start_index {
                            // If there's a string to yield then yield it...
                            return Some(&self.src[start_index..curr_index]);
                        } else {
                            // ... otherwise just move to the next mode.
                            mode = Mode::InEsc;
                        }
                    }
                }

                // In the last iteration we had the initial ESC of an escape code.
                Mode::InEsc => {
                    mode = match curr_char {
                        // For these we just await the ST (String Terminator)
                        DCS => Mode::AwaitSt,
                        SOC => Mode::AwaitSt,
                        PM => Mode::AwaitSt,
                        APC => Mode::AwaitSt,
                        // OSC (Operating System Command) has some special handling for BEL or ST
                        OSC => Mode::InOsc,
                        // Next is a CSI (Control Sequence Indicator)
                        CSI => Mode::InCsi,
                        // Another ESC?
                        ESC => {
                            // Skip the last one
                            start_index = curr_index;
                            Mode::InEsc
                        }
                        // Not really defined...
                        _ => {
                            // Just ignore the ESC I guess?
                            start_index = curr_index;
                            Mode::Normal
                        }
                    };
                }

                Mode::InCsi => {
                    // https://w.wiki/Bk2X#Control_Sequence_Introducer_commands
                    if curr_char >= '@' && curr_char <= '~' {
                        // got the "final byte": switch back to Normal mode.
                        start_index = end_index;
                        mode = Mode::Normal;
                    }
                }

                Mode::InOsc => {
                    mode = match curr_char {
                        // BEL is magic end marker for OSC too.
                        BEL => {
                            start_index = end_index;
                            Mode::Normal
                        }
                        // Maybe about to get ST end?
                        ESC => Mode::OscMaybeSt,
                        _ => Mode::InOsc,
                    };
                }

                // Are we waiting on a String Termination (ST) char?
                Mode::AwaitSt => {
                    mode = match curr_char {
                        ESC => Mode::MaybeSt,
                        _ => Mode::AwaitSt,
                    };
                }

                Mode::OscMaybeSt => {
                    mode = match curr_char {
                        // Got ST end: back to normal
                        ST_CHAR | BEL => {
                            start_index = end_index;
                            Mode::Normal
                        }
                        ESC => {
                            // Another ESC? Do nothing.
                            Mode::OscMaybeSt
                        }
                        // Undefined? Wait for next time?
                        _ => Mode::InOsc,
                    };
                }

                Mode::MaybeSt => {
                    mode = match curr_char {
                        ST_CHAR => {
                            start_index = end_index;
                            Mode::Normal
                        }
                        // Nope: back to waiting
                        _ => Mode::AwaitSt,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standard_test(sample: &str, expected: Vec<&str>) {
        let actual: Vec<&str> = sample.non_esc().collect();
        assert_eq!(expected, actual);
    }

    #[test]
    fn empty() {
        standard_test("", vec![])
    }

    #[test]
    fn single_good() {
        standard_test("a", vec!["a"])
    }

    #[test]
    fn plain_string() {
        standard_test("Hello, world!", vec!["Hello, world!"])
    }

    #[test]
    fn formatted_word() {
        standard_test(
            &format!("Hello, {ESC}{CSI}0mworld{ESC}{CSI}123m!"),
            vec!["Hello, ", "world", "!"],
        )
    }

    #[test]
    fn single_esc() {
        standard_test(&format!("{ESC}"), vec![])
    }

    #[test]
    fn multi_esc() {
        standard_test(&format!("{ESC}{ESC}"), vec![])
    }

    #[test]
    fn single_csi() {
        standard_test(&format!("{ESC}[m"), vec![])
    }

    #[test]
    fn single_csi_long() {
        standard_test(&format!("{ESC}[1;2;3m"), vec![])
    }

    #[test]
    fn front_loose_esc_single_csi() {
        standard_test(&format!("{ESC}{ESC}[m"), vec![])
    }

    #[test]
    fn back_loose_esc_single_csi() {
        standard_test(&format!("{ESC}[m{ESC}"), vec![])
    }

    #[test]
    fn csi_then_char() {
        standard_test(&format!("{ESC}[mn"), vec!["n"])
    }

    #[test]
    fn csi_long_then_char() {
        standard_test(&format!("{ESC}[1;2;3mn"), vec!["n"])
    }

    #[test]
    fn csi_char_csi() {
        standard_test(&format!("{ESC}[mn{ESC}[m"), vec!["n"])
    }

    #[test]
    fn char_csi_char() {
        standard_test(&format!("o{ESC}[mn"), vec!["o", "n"])
    }

    #[test]
    fn char_then_csi() {
        standard_test(&format!("n{ESC}[m"), vec!["n"])
    }

    #[test]
    fn partial_csi() {
        standard_test(&format!("{ESC}["), vec![])
    }

    #[test]
    fn char_then_partial_csi() {
        standard_test(&format!("n{ESC}["), vec!["n"])
    }

    #[test]
    fn osc_bel() {
        standard_test(&format!("n{ESC}]{BEL}m"), vec!["n", "m"])
    }

    #[test]
    fn osc_st() {
        standard_test(&format!("n{ESC}]{ESC}{ST}m"), vec!["n", "m"])
    }

    #[test]
    fn osc_errant_esc_st() {
        standard_test(&format!("n{ESC}]{ESC}{ST}m"), vec!["n", "m"])
    }

    #[test]
    fn osc_errant_esc_bel() {
        standard_test(&format!("n{ESC}]{ESC}{BEL}m"), vec!["n", "m"])
    }
}

