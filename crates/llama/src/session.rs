use {
    crate::{owned_ptr::OwnedPtr, sys, Model},
    std::{any, fmt, slice},
};

/// An inference session.
pub struct Session {
    pub(crate) session_ptr: OwnedPtr,
    pub(crate) sampler_ptr: OwnedPtr,
    model: Model,
}

/// Options and flags which can be used to configure how a session is created.
pub struct SessionOptions {
    pub(crate) options_ptr: OwnedPtr,
    pub(crate) sampler_options_ptr: OwnedPtr,
}

pub struct SessionBatch {
    pub(crate) batch_ptr: OwnedPtr,
    pub(crate) capacity: u32,
}

impl SessionBatch {
    pub fn new(token_capacity: u32, embedding_size: u32, max_sequence_ids: u32) -> Self {
        unsafe {
            Self {
                batch_ptr: OwnedPtr::new(
                    sys::bindings_session_batch_init(
                        token_capacity,
                        embedding_size,
                        max_sequence_ids,
                    ),
                    sys::bindings_session_batch_drop,
                ),
                capacity: token_capacity,
            }
        }
    }

    pub fn add_token(&mut self, token: i32, index: u32, logits: bool) {
        unsafe {
            sys::bindings_session_batch_add_token(
                self.batch_ptr.as_mut_ptr(),
                token,
                index,
                logits,
            );
        }
    }

    pub fn clear(&mut self) {
        unsafe {
            sys::bindings_session_batch_clear(self.batch_ptr.as_mut_ptr());
        }
    }

    pub fn len(&self) -> usize {
        unsafe {
            sys::bindings_session_batch_tokens_len(self.batch_ptr.as_ptr())
                .try_into()
                .unwrap()
        }
    }

    pub fn tokens(&self) -> &[i32] {
        unsafe {
            slice::from_raw_parts(
                sys::bindings_session_batch_tokens_ptr(self.batch_ptr.as_ptr()),
                self.len(),
            )
        }
    }

    pub fn tokens_mut(&mut self) -> &mut [i32] {
        unsafe {
            slice::from_raw_parts_mut(
                sys::bindings_session_batch_tokens_mut_ptr(self.batch_ptr.as_mut_ptr()),
                self.len(),
            )
        }
    }

    pub fn logits(&self) -> &[bool] {
        unsafe {
            slice::from_raw_parts(
                sys::bindings_session_batch_logits_ptr(self.batch_ptr.as_ptr()).cast(),
                self.len(),
            )
        }
    }

    pub fn logits_mut(&mut self) -> &mut [bool] {
        unsafe {
            slice::from_raw_parts_mut(
                sys::bindings_session_batch_logits_mut_ptr(self.batch_ptr.as_mut_ptr()).cast(),
                self.len(),
            )
        }
    }
}

impl Session {
    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn into_model(self) -> Model {
        self.model
    }

    pub fn decode(&mut self, batch: &mut SessionBatch) {
        unsafe {
            sys::bindings_session_decode(
                self.session_ptr.as_mut_ptr(),
                batch.batch_ptr.as_mut_ptr(),
            );
        }
    }

    pub fn sample(&mut self) -> i32 {
        unsafe {
            sys::bindings_session_sampler_sample(
                self.sampler_ptr.as_mut_ptr(),
                self.session_ptr.as_mut_ptr(),
            )
        }
    }

    pub fn accept(&mut self, token: i32) {
        unsafe {
            sys::bindings_session_sampler_accept(
                self.sampler_ptr.as_mut_ptr(),
                self.session_ptr.as_mut_ptr(),
                token,
            );
        }
    }

    pub fn infer(&mut self, tokens: &[i32]) -> String {
        let mut batch = SessionBatch::new(512, 0, 1);

        for (index, token) in tokens.iter().copied().enumerate() {
            batch.add_token(token, index as u32, false);
        }

        if let Some(logit) = batch.logits_mut().last_mut() {
            *logit = true;
        }

        let mut tokens = Vec::new();

        for i in tokens.len()..100 {
            self.decode(&mut batch);

            let token = self.sample();

            self.accept(token);

            batch.clear();
            batch.add_token(token, i as u32, true);
            tokens.push(token);
        }

        let mut string = String::new();

        self.model.detokenize(&tokens, &mut string);

        string
    }
}

impl SessionOptions {
    /// Creates a new set of session options ready for configuration.
    pub fn new() -> Self {
        unsafe {
            Self {
                options_ptr: OwnedPtr::new(
                    sys::bindings_session_options_new(),
                    sys::bindings_session_options_drop,
                ),
                sampler_options_ptr: OwnedPtr::new(
                    sys::bindings_session_sampler_options_new(),
                    sys::bindings_session_sampler_options_drop,
                ),
            }
        }
    }

    /// Creates a session with the specified model.
    pub fn with_model(mut self, mut model: Model) -> Session {
        unsafe {
            Session {
                session_ptr: OwnedPtr::new(
                    sys::bindings_session_new(
                        model.model_ptr.as_mut_ptr(),
                        self.options_ptr.as_ptr(),
                    ),
                    sys::bindings_session_drop,
                ),
                sampler_ptr: OwnedPtr::new(
                    sys::bindings_session_sampler_new(self.sampler_options_ptr.as_mut_ptr()),
                    sys::bindings_session_sampler_drop,
                ),
                model,
            }
        }
    }
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for SessionOptions {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct(any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for Session {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct(any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
