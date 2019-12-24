use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AuthenticationMethod {
    NoAuthentication,
    UsernamePassword,
}

pub enum Authentication {
    UsernamePassword { user_name: Vec<u8>, password: Vec<u8> },
    Token { token: Vec<u8> },
}

#[derive(Debug)]
pub struct AuthenticationManager {
    user_list: HashMap<Vec<u8>, Vec<u8>>,
}

impl AuthenticationManager {
    pub fn new() -> AuthenticationManager {
        AuthenticationManager { user_list: HashMap::default() }
    }

    pub fn supported_method(&self, _addr: &SocketAddr) -> AuthenticationMethod {
        AuthenticationMethod::NoAuthentication
    }

    pub async fn authenticate(&self, auth: Authentication) -> bool {
        match auth {
            Authentication::UsernamePassword { user_name, password } => {
                match self.user_list.get(&user_name) {
                    Some(passwd) => return passwd == &password,
                    None => false,
                }
            }
            Authentication::Token { token } => {
                let _ = token;
                // TODO
                false
            }
        }
    }
}
