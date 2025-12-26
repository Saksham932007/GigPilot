pub mod embeddings;
pub mod search;

pub use embeddings::{store_embedding, Embedding};
pub use search::search_similar_projects;
