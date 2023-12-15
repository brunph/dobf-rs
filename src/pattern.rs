use std::u8;
use lazy_static::lazy_static;
use std::collections::HashMap;

// TODO: possibly use https://github.com/rust-phf/rust-phf instead?
lazy_static! {
    static ref NOPMAP: HashMap<usize, Vec<u8>> = {
        let mut m = HashMap::new();
        m.insert(2, vec![0x66, 0x90]); // becomes xchg ax, ax which is a 2 byte nop
        m.insert(3, vec![0x0f, 0x1f, 0x00]); // nop dword ptr [rax]
        m.insert(4, vec![0x0f, 0x1f, 0x40, 0x00]);  // nop dword ptr [rax + 00]
        m.insert(5, vec![0x0f, 0x1f, 0x44, 0x00, 0x00]); // nop dword ptr [rax + rax + 00]
        m.insert(6, vec![0x66, 0x0f, 0x1f, 0x44, 0x00, 0x00]); // nop word ptr [rax + rax + 00]
        m.insert(7, vec![0x0f, 0x1f, 0x80, 0x00, 0x00, 0x00, 0x00]); // nop dword ptr [rax + 00000000]
        m.insert(8, vec![0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00]); // nop dword ptr [rax + rax + 00000000]
        m.insert(9, vec![0x66, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00]); // nop word ptr [rax + rax + 00000000]
        m
    };
}

pub struct PatternBuilder {
    pat: String,
    should_simplify_nops: bool,
}

impl PatternBuilder {
    pub fn new(pattern: &str) -> PatternBuilder {
        PatternBuilder {
            pat: String::from(pattern),
            should_simplify_nops: false,
        }
    }

    pub fn simplify_nops(mut self) -> PatternBuilder {
        self.should_simplify_nops = true;
        self
    }

    pub fn build(self) -> Option<Pattern> {
        Pattern::new(&self.pat, self.should_simplify_nops)
    }
}

pub struct Pattern {
    pattern: Vec<u8>,
    wildcard_locations: Vec<usize>,
}


impl Pattern {
    pub fn new(pattern: &str, simplfynops: bool) -> Option<Pattern> {
        let chunks: Vec<&str> = pattern.split(' ').collect();
        if chunks.is_empty() {
            return None;
        }

        let finalized = chunks.iter().map(|b| if b.contains('?') { 0 } else { u8::from_str_radix(b, 16).unwrap() }).collect::<Vec<u8>>();
        let mut p = Pattern {
            pattern: finalized,
            wildcard_locations: chunks.iter().enumerate().filter(|(_, &b)| b.contains('?')).map(|(i, _)| i).collect::<Vec<usize>>(),
        };

        if simplfynops {
            p.simplify_nops();
        }

        Some(p)
    }

    pub fn builder(pattern: &str) -> PatternBuilder {
        PatternBuilder::new(pattern)
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Pattern> {
        if bytes.is_empty() {
            return None;
        }
        
        Some(Pattern {
            pattern: bytes.to_vec(),
            wildcard_locations: bytes.iter().enumerate().filter(|(_, &b)| b == 0).map(|(i, _)| i).collect::<Vec<usize>>(),
        })
    }

    pub fn print(&self) {
        let pat_str = self.pattern.iter().map(|b| if *b != 0 { format!("{:02X}", b) } else { String::from("?") }).collect::<Vec<String>>().join(" ");
        println!("{}", pat_str);
    }

    pub fn is_empty(&self) -> bool {
        self.pattern.is_empty()
    }

    pub fn len(&self) -> usize {
        self.pattern.len()
    }

    pub fn iter(&self) -> std::slice::Iter<u8> {
        self.pattern.iter()
    }

    /// Checks if the given slice of bytes matches the pattern. If so, returns the index of the first match.
    pub fn matches(&self, data: &[u8]) -> Option<usize> {
        if data.len() < self.len() {
            return None;
        }
    
        data.windows(self.len()).position(|window| {
            window.iter().zip(self.iter()).all(|(&left, &right)| right == 0u8 || left == right)
        })
    }

    pub fn is_wildcard(&self, index: usize) -> bool {
        self.wildcard_locations.contains(&index)
    }

    pub fn matches_all(&self, data: &[u8]) -> Vec<usize> {
        if data.len() < self.len() {
            return Vec::new();
        }
    
        data.windows(self.len()).enumerate().filter(|(_, window)| window.iter().zip(self.iter()).enumerate().all(|(j, (&left, &right))| self.is_wildcard(j) || left == right)).map(|(i, _)| i).collect::<Vec<usize>>()
    }

    // TODO: fix this mess of a function
    pub fn simplify_nops(&mut self) -> &mut Pattern {
        let mut new_pattern = self.pattern.clone();

        let nop_blocks: Vec<Vec<usize>> = self
            .iter()
            .enumerate()
            .filter(|&(_, &b)| b == 0x90)
            .fold(Vec::new(), |mut result, (i, _)| {
                if let Some(last_block) = result.last_mut() {
                    if last_block.last().map_or(false, |&last_index| last_index == i - 1) {
                        last_block.push(i);
                    } else {
                        result.push(vec![i]);
                    }
                } else {
                    result.push(vec![i]);
                }
                result
        });

        for block in nop_blocks.iter().flat_map(|inner_vec| inner_vec.chunks(8).map(|chunk| chunk.to_vec())) {
            if NOPMAP.contains_key(&block.len()) {
                let replacement = NOPMAP.get(&block.len()).unwrap();
                new_pattern.splice(block.first().unwrap().to_owned()..block.last().unwrap().to_owned() + 1, replacement.to_owned());
            }
        }
        assert_eq!(self.pattern.len(), new_pattern.len());

        self.pattern = new_pattern;
        self
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern() {
        let pat = Pattern::builder("83 3D ? ? ? ? ? 75 ? 8B 43").build().unwrap();

        let data = vec![0x83, 0x3D, 0x32, 0x00, 0x00, 0x00, 0x00, 0x75, 0x00, 0x8B, 0x43];

        let matches = pat.matches_all(&data);

        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_simplify_nops() {
        let pat = Pattern::builder("90 90 90 90 90").simplify_nops().build().unwrap();

        assert_eq!(pat.iter().as_slice(), NOPMAP.get(&5).unwrap().to_owned());
    }

    #[test]
    fn test_simplify_nops_alt_char() {
        let pat = Pattern::builder("55 90 90 90 90 90 90 90 8B 90").simplify_nops().build().unwrap();

        assert_eq!(pat.iter().as_slice(), [0x55, 0x0F, 0x1F, 0x80, 0x00, 0x00, 0x00, 0x00, 0x8B, 0x90]);
    }
}
