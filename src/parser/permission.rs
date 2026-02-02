use nom::{
    IResult, Parser,
    character::complete::{char, digit1},
    combinator::{map_res, opt},
    sequence::preceded,
};

pub fn parse_permissions_str(input: &str) -> eyre::Result<u32> {
    fn permissions_parser(input: &str) -> IResult<&str, u32> {
        let parse_leading_zero = opt(char('0'));
        let parser = preceded(parse_leading_zero, digit1);
        map_res(parser, |octal_str: &str| u32::from_str_radix(octal_str, 8)).parse(input)
    }

    match permissions_parser(input) {
        Ok((_, mode)) => Ok(mode),
        Err(err) => Err(eyre::eyre!("Failed to parse permissions: {:?}", err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_permission_string() {
        for (s, r) in [("0700", "700"), ("700", "700"), ("400", "400")] {
            assert_eq!(
                parse_permissions_str(s).unwrap(),
                u32::from_str_radix(r, 8).unwrap()
            );
        }
        assert!(parse_permissions_str("33993").is_err(),);
        assert!(parse_permissions_str("0000111").is_ok(),);
        assert!(parse_permissions_str("1000119").is_err(),);
    }
}
