pub fn pad(ch: char, nbsp: bool) -> &'static str {
    match ch {
        '0' => "0",
        '?' => {
            if nbsp {
                "\u{00A0}"
            } else {
                " "
            }
        }
        _ => "",
    }
}
