use nom::{
    IResult, Parser,
    bytes::complete::{tag, take_while_m_n},
    combinator::verify,
    error::Error,
    sequence::delimited,
};

// 4 braces + 64 bytes hash + 2 whitespace
const PLACEHOLDER_LENGTH: usize = 70;

fn parse_braced_hash(input: &str) -> IResult<&str, &str, Error<&str>> {
    delimited(
        tag("{{ "),
        take_while_m_n(64, 64, |c: char| c.is_ascii_hexdigit()),
        tag(" }}"),
    )
    .parse(input)
}

pub fn extract_all_hashes<'a>(input: &'a str, res: &mut Vec<&'a str>) {
    if input.len() < PLACEHOLDER_LENGTH {
        // less than expected `{{ hash }}` length
        return;
    }
    if let Ok((o, b)) = verify(parse_braced_hash, |_: &str| true).parse(input) {
        res.push(b);
        extract_all_hashes(o, res)
    } else {
        let this = {
            // handle multibytes
            let res = input.char_indices().nth(1).map_or("", |(i, _)| &input[i..]);
            // skip to next `{`
            res.find('{').map_or("", |index| &res[index..])
        };

        extract_all_hashes(this, res)
    }
}
