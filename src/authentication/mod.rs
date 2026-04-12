use std::{collections::HashMap, net::SocketAddr};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticationMethod {
    NoAuthentication,
    UsernamePassword,
}

pub enum Authentication {
    UsernamePassword { user_name: Vec<u8>, password: Vec<u8> },
    Token { token: Vec<u8> },
}

#[derive(Debug, Default)]
pub struct AuthenticationManager {
    user_list: HashMap<Vec<u8>, Vec<u8>>,
}

impl AuthenticationManager {
    #[inline]
    #[must_use]
    pub fn new() -> Self { Self { user_list: HashMap::default() } }

    #[inline]
    #[must_use]
    pub const fn supported_method(&self, _addr: &SocketAddr) -> AuthenticationMethod {
        AuthenticationMethod::NoAuthentication
    }

    #[must_use]
    pub fn authenticate(&self, auth: Authentication) -> bool {
        match auth {
            Authentication::UsernamePassword { user_name, password } => {
                self.user_list.get(&user_name) == Some(&password)
            }
            Authentication::Token { .. } => {
                // TODO: implement token-based authentication
                false
            }
        }
    }
}
