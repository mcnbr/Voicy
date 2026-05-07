use anyhow::Result;
use candle_core::{Device, Tensor, IndexOp};
use candle_transformers::models::whisper::{self as m, audio, Config};
use tokenizers::Tokenizer;
use hf_hub::{api::sync::ApiBuilder, Cache};
use std::sync::Mutex;

pub fn token_id(tokenizer: &Tokenizer, token: &str) -> candle_core::Result<u32> {
    match tokenizer.token_to_id(token) {
        None => candle_core::bail!("no token-id for {}", token),
        Some(id) => Ok(id),
    }
}

pub struct WhisperModel {
    model: Mutex<Option<m::quantized_model::Whisper>>,
}
