use confique::Config;

#[derive(Config)]
pub struct Conf {
    #[config(nested)]
    pub source: ImmichSource,
}

#[derive(Config)]
pub struct ImmichSource {
    pub url: String,
    pub api_key: String,
}
