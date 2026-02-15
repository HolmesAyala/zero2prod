use linkify::{LinkFinder, LinkKind};

pub fn get_first_link(value: &str) -> String {
    let links: Vec<_> = LinkFinder::new()
        .links(value)
        .filter(|link| *link.kind() == LinkKind::Url)
        .collect();

    assert_eq!(
        links.len(),
        1,
        "There are not at least one link in the value provided. value = {}",
        value
    );

    links[0].as_str().to_owned()
}
