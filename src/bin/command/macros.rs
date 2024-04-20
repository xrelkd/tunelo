macro_rules! merge_option_field {
    ($config:ident, $opt:ident) => {
        $opt.take().map(|opt| $config.$opt = opt);
    };
}

macro_rules! impl_config_load {
    ($config:ident) => {
        pub fn load<P: AsRef<Path>>(path: P) -> Result<$config, Error> {
            let content = std::fs::read_to_string(&path).map_err(|source| {
                Error::ReadConfigFile { source, file_path: path.as_ref().to_owned() }
            })?;

            let config = Self::from_toml(&content)?;
            Ok(config)
        }

        pub fn from_toml(content: &str) -> Result<$config, Error> {
            toml::from_str(&content).map_err(|source| Error::ParseConfigFromToml { source })
        }
    };
}
