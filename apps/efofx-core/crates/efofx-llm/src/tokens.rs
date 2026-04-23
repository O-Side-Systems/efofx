//! Token counting for budget enforcement. Uses `tiktoken-rs` to derive the
//! BPE encoder for OpenAI models. We don't call the OpenAI tokenizer API —
//! this is all local.

use thiserror::Error;
use tiktoken_rs::bpe_for_model;

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("no tokenizer available for model `{0}`")]
    UnknownModel(String),
}

/// Count tokens for a single string under `model`'s encoding.
pub fn count_tokens(model: &str, text: &str) -> Result<usize, TokenError> {
    let bpe = bpe_for_model(model).map_err(|_| TokenError::UnknownModel(model.to_string()))?;
    Ok(bpe.encode_with_special_tokens(text).len())
}

/// Approximate token count for a chat conversation. Replicates OpenAI's
/// documented overhead (~4 tokens per message + ~2 trailing) so budget
/// checks err on the high side. Good enough for enforcement; the actual
/// billed count comes back via `Usage` on the completion response.
pub fn count_chat_tokens(
    model: &str,
    messages: &[(crate::types::Role, &str)],
) -> Result<usize, TokenError> {
    let bpe = bpe_for_model(model).map_err(|_| TokenError::UnknownModel(model.to_string()))?;
    // Per OpenAI cookbook: every message carries ~4 overhead tokens for
    // `<im_start>role\ncontent<im_end>\n` framing.
    const PER_MESSAGE: usize = 4;
    const TRAILING: usize = 2;
    let mut total = 0usize;
    for (role, content) in messages {
        total += PER_MESSAGE;
        total += bpe.encode_with_special_tokens(role.as_str()).len();
        total += bpe.encode_with_special_tokens(content).len();
    }
    Ok(total + TRAILING)
}

impl crate::types::Role {
    fn as_str(&self) -> &'static str {
        match self {
            crate::types::Role::System => "system",
            crate::types::Role::User => "user",
            crate::types::Role::Assistant => "assistant",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Role;

    #[test]
    fn counts_simple_text() {
        // "hello world" under gpt-4 BPE is a small, stable number. Assert
        // bounds rather than an exact value so tokenizer updates don't break
        // us — we just need confidence the counter works.
        let n = count_tokens("gpt-4o-mini", "hello world").unwrap();
        assert!((1..=10).contains(&n), "unexpected token count: {n}");
    }

    #[test]
    fn chat_count_adds_per_message_overhead() {
        let short = count_chat_tokens("gpt-4o-mini", &[(Role::User, "hi")]).unwrap();
        let longer = count_chat_tokens(
            "gpt-4o-mini",
            &[(Role::System, "be concise"), (Role::User, "hi")],
        )
        .unwrap();
        // Adding one more message should add at least PER_MESSAGE (4) tokens
        // on top of the content tokens themselves.
        assert!(longer > short + 4);
    }

    #[test]
    fn unknown_model_errors() {
        let err = count_tokens("not-a-real-model-xyz", "hi").unwrap_err();
        assert!(matches!(err, TokenError::UnknownModel(_)));
    }
}
