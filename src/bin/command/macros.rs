macro_rules! merge_option_field {
    ($config:ident, $opt:ident) => {
        $opt.take().map(|opt| $config.$opt = opt);
    };
}

macro_rules! impl_config_load {
    ($config:ident) => {
        pub fn load<P: AsRef<Path>>(path: P) -> Result<$config, Error> {
            let content = std::fs::read(&path).map_err(|source| Error::ReadConfigFile {
                source,
                file_path: path.as_ref().to_owned(),
            })?;
            let config = toml::from_slice(&content).map_err(|source| Error::DeserializeConfig {
                source,
                file_path: path.as_ref().to_owned(),
            })?;
            Ok(config)
        }
    };
}
