//! Simple BM25 index for lexical retrieval over graph nodes.

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Bm25Index {
    /// term -> (node_id -> frequency)
    index: HashMap<String, HashMap<String, u32>>,
    /// document lengths
    doc_lengths: HashMap<String, usize>,
    avg_doc_len: f32,
    total_docs: usize,
}

impl Bm25Index {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_document(&mut self, doc_id: String, text: &str) {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let len = tokens.len();

        self.doc_lengths.insert(doc_id.clone(), len);
        self.total_docs += 1;

        let mut freqs = HashMap::new();
        for token in tokens {
            let t = token.to_lowercase();
            *freqs.entry(t.clone()).or_insert(0) += 1;
            self.index.entry(t).or_default().insert(doc_id.clone(), *freqs.get(&t).unwrap());
        }

        // update average doc length
        let sum: usize = self.doc_lengths.values().sum();
        self.avg_doc_len = sum as f32 / self.total_docs as f32;
    }

    pub fn score(&self, query: &str, doc_id: &str) -> f32 {
        let k1 = 1.5;
        let b = 0.75;
        let mut score = 0.0;

        for term in query.split_whitespace().map(|s| s.to_lowercase()) {
            if let Some(postings) = self.index.get(&term) {
                if let Some(&tf) = postings.get(doc_id) {
                    let idf = ((self.total_docs as f32 - postings.len() as f32 + 0.5)
                        / (postings.len() as f32 + 0.5))
                        .ln();
                    let len = *self.doc_lengths.get(doc_id).unwrap_or(&1) as f32;
                    let norm = 1.0 - b + b * (len / self.avg_doc_len);
                    score += idf * (tf as f32 * (k1 + 1.0)) / (tf as f32 + k1 * norm);
                }
            }
        }
        score
    }
}
