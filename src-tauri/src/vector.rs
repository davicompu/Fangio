//! Vector helpers: float ↔ bytes, cosine similarity, top-k.
//!
//! At MVP scale (hundreds of articles, tens of chunks each = a few thousand
//! vectors per project) brute-force cosine in Rust is faster and simpler than
//! shipping a vector index. Embeddings live in SQLite as BLOBs.

pub fn f32s_to_bytes(v: &[f32]) -> Vec<u8> {
    bytemuck::cast_slice(v).to_vec()
}

pub fn bytes_to_f32s(b: &[u8]) -> Vec<f32> {
    if b.len() % 4 != 0 {
        return vec![];
    }
    bytemuck::cast_slice(b).to_vec()
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

#[derive(Debug)]
pub struct Scored<T> {
    pub item: T,
    pub score: f32,
}

pub fn top_k<T>(items: Vec<(T, Vec<f32>)>, query: &[f32], k: usize) -> Vec<Scored<T>> {
    let mut scored: Vec<Scored<T>> = items
        .into_iter()
        .map(|(item, emb)| Scored {
            score: cosine(&emb, query),
            item,
        })
        .collect();
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    scored
}
