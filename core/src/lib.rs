pub fn versao() -> &'static str {
    "X360 Nexus Core 0.4"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teste_versao() {
        assert_eq!(versao(), "X360 Nexus Core 0.4");
    }
}
