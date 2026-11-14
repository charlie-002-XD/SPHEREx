//! Minimal VOTable (XML) reader for SIA2 responses.
//!
//! IRSA emits VOTable 1.3 with `<TABLE><FIELD .../><DATA><TABLEDATA>
//! <TR><TD>..</TD></TR>..`. This is a small pull scanner, not a general
//! XML parser: it understands elements, attributes (single/double
//! quoted), character data, CDATA sections, comments, processing
//! instructions and `<!...>` declarations (including a DOCTYPE internal
//! subset), and decodes the five predefined entities plus `&#NN;` /
//! `&#xNN;` numeric references. Namespace prefixes are stripped, so
//! prefixed and unprefixed VOTable documents both parse.

use super::error::IrsaError;

/// One `<FIELD>` of a table.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// `name` attribute (as emitted by IRSA, e.g. `t_min`).
    pub name: String,
    /// `unit` attribute (e.g. `m`, `d`, `deg`), if present.
    pub unit: Option<String>,
}

/// One parsed `<TABLE>`: field metadata plus raw `<TD>` cell text.
/// Rows shorter than `fields` are right-padded with empty strings.
#[derive(Debug, Clone, PartialEq)]
pub struct Votable {
    /// Column metadata in column order.
    pub fields: Vec<Field>,
    /// Rows of cell text (entity-decoded, untrimmed).
    pub rows: Vec<Vec<String>>,
}

impl Votable {
    /// Index of the field named `name` (case-insensitive), if any.
    pub fn column(&self, name: &str) -> Option<usize> {
        self.fields
            .iter()
            .position(|f| f.name.eq_ignore_ascii_case(name))
    }

    /// Text of cell `col` in `row`, trimmed; `None` when absent/empty.
    pub fn cell<'r>(&self, row: &'r [String], col: Option<usize>) -> Option<&'r str> {
        col.and_then(|i| row.get(i))
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
    }

    /// The `unit` attribute of the field named `name`.
    pub fn unit(&self, name: &str) -> Option<&str> {
        self.column(name)
            .and_then(|i| self.fields[i].unit.as_deref())
    }
}

/// SAX-like event produced by [`Scanner`].
#[derive(Debug, PartialEq)]
enum Event {
    /// `<name attrs..>` (`empty` for `<name/>`).
    Start {
        /// Local tag name (prefix stripped).
        name: String,
        /// `(attr, value)` pairs, entity-decoded.
        attrs: Vec<(String, String)>,
        /// Self-closing element.
        empty: bool,
    },
    /// `</name>`.
    End {
        /// Local tag name.
        name: String,
    },
    /// Character data or CDATA content, entity-decoded.
    Text(String),
}

/// Cursor over the XML source.
struct Scanner<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn new(s: &'a str) -> Self {
        Self { s, pos: 0 }
    }

    /// Next event; `Ok(None)` at end of input.
    fn next(&mut self) -> Result<Option<Event>, IrsaError> {
        let rest = &self.s[self.pos..];
        if rest.is_empty() {
            return Ok(None);
        }
        if !rest.starts_with('<') {
            // Character data up to the next tag.
            let end = rest.find('<').unwrap_or(rest.len());
            self.pos += end;
            return Ok(Some(Event::Text(decode_entities(&rest[..end]))));
        }
        if let Some(r) = rest.strip_prefix("<!--") {
            // Comment: skip to `-->`.
            let end = r
                .find("-->")
                .ok_or_else(|| IrsaError::Parse("unterminated XML comment".into()))?;
            self.pos += 4 + end + 3;
            return self.next();
        }
        if let Some(r) = rest.strip_prefix("<![CDATA[") {
            let end = r
                .find("]]>")
                .ok_or_else(|| IrsaError::Parse("unterminated CDATA section".into()))?;
            self.pos += 9 + end + 3;
            return Ok(Some(Event::Text(r[..end].to_string())));
        }
        if let Some(r) = rest.strip_prefix("<?") {
            // Processing instruction: skip to `?>`.
            let end = r
                .find("?>")
                .ok_or_else(|| IrsaError::Parse("unterminated processing instruction".into()))?;
            self.pos += 2 + end + 2;
            return self.next();
        }
        if rest.starts_with("<!") {
            // DOCTYPE or other declaration: skip to `>` not inside a
            // bracketed internal subset or quoted literal.
            self.skip_declaration()?;
            return self.next();
        }
        self.tag()
    }

    /// Skip `<!...>` declarations; tolerates `[ <!ENTITY ..> ]` subsets
    /// and quoted literals inside them.
    fn skip_declaration(&mut self) -> Result<(), IrsaError> {
        let bytes = self.s.as_bytes();
        let mut i = self.pos + 2;
        let mut bracket = 0usize;
        let mut quote = 0u8;
        while i < bytes.len() {
            let b = bytes[i];
            if quote != 0 {
                if b == quote {
                    quote = 0;
                }
            } else {
                match b {
                    b'\'' | b'"' => quote = b,
                    b'[' => bracket += 1,
                    b']' => bracket = bracket.saturating_sub(1),
                    b'>' if bracket == 0 => {
                        self.pos = i + 1;
                        return Ok(());
                    }
                    _ => {}
                }
            }
            i += 1;
        }
        Err(IrsaError::Parse("unterminated XML declaration".into()))
    }

    /// Parse an open/close tag starting at `self.pos` (`<` or `</`).
    fn tag(&mut self) -> Result<Option<Event>, IrsaError> {
        let bytes = self.s.as_bytes();
        let mut i = self.pos + 1;
        let closing = bytes.get(i) == Some(&b'/');
        if closing {
            i += 1;
        }
        i = skip_ws(bytes, i);
        let name_start = i;
        while i < bytes.len() && is_name_byte(bytes[i]) {
            i += 1;
        }
        let raw_name = &self.s[name_start..i];
        if raw_name.is_empty() {
            let ctx: String = self.s[self.pos..].chars().take(24).collect();
            return Err(IrsaError::Parse(format!(
                "malformed XML tag at byte {}: {ctx:?}",
                self.pos
            )));
        }
        let name = local_name(raw_name).to_string();
        if closing {
            while i < bytes.len() && bytes[i] != b'>' {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(IrsaError::Parse("unterminated closing tag".into()));
            }
            self.pos = i + 1;
            return Ok(Some(Event::End { name }));
        }

        // Attributes until `>` or `/>`.
        let mut attrs = Vec::new();
        let mut empty = false;
        loop {
            i = skip_ws(bytes, i);
            match bytes.get(i) {
                Some(&b'>') => {
                    i += 1;
                    break;
                }
                Some(&b'/') => {
                    if bytes.get(i + 1) == Some(&b'>') {
                        empty = true;
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                None => {
                    let ctx: String = self.s[self.pos..].chars().take(24).collect();
                    return Err(IrsaError::Parse(format!(
                        "unterminated tag at byte {}: {ctx:?}",
                        self.pos
                    )));
                }
                _ => {
                    let (key, val, next) = self.attribute(i)?;
                    attrs.push((key, val));
                    i = next;
                }
            }
        }
        self.pos = i;
        Ok(Some(Event::Start { name, attrs, empty }))
    }

    /// Parse one `name="value"` (or `'value'` / bare) attribute at `i`.
    fn attribute(&self, i: usize) -> Result<(String, String, usize), IrsaError> {
        let bytes = self.s.as_bytes();
        let mut j = i;
        while j < bytes.len() && is_name_byte(bytes[j]) {
            j += 1;
        }
        let key = local_name(&self.s[i..j]).to_string();
        j = skip_ws(bytes, j);
        if bytes.get(j) != Some(&b'=') {
            // Valueless attribute (legal XML for some decls); keep empty.
            return Ok((key, String::new(), j));
        }
        j += 1;
        j = skip_ws(bytes, j);
        let val;
        match bytes.get(j) {
            Some(q @ (b'"' | b'\'')) => {
                let q = *q;
                let start = j + 1;
                let mut k = start;
                while k < bytes.len() && bytes[k] != q {
                    k += 1;
                }
                if k >= bytes.len() {
                    return Err(IrsaError::Parse("unterminated attribute value".into()));
                }
                val = self.s[start..k].to_string();
                j = k + 1;
            }
            Some(_) => {
                // Unquoted value: runs to whitespace or `>`.
                let start = j;
                while j < bytes.len()
                    && !bytes[j].is_ascii_whitespace()
                    && bytes[j] != b'>'
                    && bytes[j] != b'/'
                {
                    j += 1;
                }
                val = self.s[start..j].to_string();
            }
            None => return Err(IrsaError::Parse("unterminated attribute".into())),
        }
        Ok((key, decode_entities(&val), j))
    }
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':')
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Strip a `prefix:` namespace qualifier.
fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

/// Decode the five predefined XML entities plus numeric references.
/// Unknown references are passed through untouched.
fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let end = tail.find(';');
        match end {
            Some(e) => {
                let entity = &tail[1..e];
                match entity {
                    "amp" => out.push('&'),
                    "lt" => out.push('<'),
                    "gt" => out.push('>'),
                    "quot" => out.push('"'),
                    "apos" => out.push('\''),
                    _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                        match u32::from_str_radix(&entity[2..], 16)
                            .ok()
                            .and_then(char::from_u32)
                        {
                            Some(c) => out.push(c),
                            None => out.push_str(&tail[..=e]),
                        }
                    }
                    _ if entity.starts_with('#') => {
                        match entity[1..].parse::<u32>().ok().and_then(char::from_u32) {
                            Some(c) => out.push(c),
                            None => out.push_str(&tail[..=e]),
                        }
                    }
                    _ => out.push_str(&tail[..=e]),
                }
                rest = &tail[e + 1..];
            }
            None => {
                out.push_str(tail);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Parse every `<TABLE>` of a VOTable document, in document order.
/// A document may contain several RESOURCE elements (IRSA prepends a
/// `type="meta"` resource describing the service); callers pick the
/// table they need, e.g. the first one with an `access_url` column.
pub fn parse_votable(xml: &str) -> Result<Vec<Votable>, IrsaError> {
    let mut tables = Vec::new();
    let mut scanner = Scanner::new(xml);

    // State for the table currently being read.
    let mut depth = 0usize; // element depth inside the active TABLE
    let mut fields: Vec<Field> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut in_table = false;
    let mut in_tabledata = false;
    let mut in_field = false; // inside a non-empty FIELD element
    let mut cur_row: Option<Vec<String>> = None;
    let mut cur_cell: Option<String> = None;

    while let Some(ev) = scanner.next()? {
        match ev {
            Event::Start { name, attrs, empty } => {
                if in_table {
                    depth += usize::from(!empty);
                }
                match name.as_str() {
                    "TABLE" if !in_table && !empty => {
                        in_table = true;
                        depth = 1;
                        fields.clear();
                        rows.clear();
                    }
                    "FIELD" if in_table && !in_tabledata && !in_field => {
                        let get = |k: &str| {
                            attrs
                                .iter()
                                .find(|(a, _)| a.eq_ignore_ascii_case(k))
                                .map(|(_, v)| v.clone())
                        };
                        fields.push(Field {
                            name: get("name").unwrap_or_default(),
                            unit: get("unit"),
                        });
                        in_field = !empty;
                    }
                    "TABLEDATA" if in_table => in_tabledata = true,
                    "TR" if in_tabledata => cur_row = Some(Vec::new()),
                    "TD" if cur_row.is_some() => {
                        if empty {
                            if let Some(row) = cur_row.as_mut() {
                                row.push(String::new());
                            }
                        } else {
                            cur_cell = Some(String::new());
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(t) => {
                if let Some(cell) = cur_cell.as_mut() {
                    cell.push_str(&t);
                }
            }
            Event::End { name } => {
                match name.as_str() {
                    "TD" => {
                        if let (Some(row), Some(cell)) = (cur_row.as_mut(), cur_cell.take()) {
                            row.push(cell);
                        }
                    }
                    "TR" => {
                        if let Some(mut row) = cur_row.take() {
                            while row.len() < fields.len() {
                                row.push(String::new());
                            }
                            rows.push(row);
                        }
                    }
                    "TABLEDATA" => in_tabledata = false,
                    "FIELD" => in_field = false,
                    _ => {}
                }
                if in_table {
                    depth = depth.saturating_sub(1);
                    if name == "TABLE" && depth == 0 {
                        in_table = false;
                        tables.push(Votable {
                            fields: std::mem::take(&mut fields),
                            rows: std::mem::take(&mut rows),
                        });
                    }
                }
            }
        }
    }
    Ok(tables)
}
