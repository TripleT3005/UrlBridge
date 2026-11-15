use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::UI::Shell::CommandLineToArgvW;

use crate::config::{self, Config};
use crate::json;
use crate::plan::{Plan, quote};
use crate::url;
use crate::value;
use crate::wide::{from_wide_pointer, to_wide};

// Constants
const ROUNDS: usize = 20_000;
const CONFIG: &str = r#"{"protocols":[
    {"scheme":"bridge-rdp","name":"Remote Desktop","program":"C:\\Program Files\\Remote Tool\\tool.exe",
     "arguments":["/v:{host}:{port}","--mode","{mode}","--id={id}","--title","Remote \"control\" \\"],
     "parameters":[{"name":"host","type":"host"},{"name":"port","type":"integer","min":1,"max":65535},
                   {"name":"mode","type":"choice","values":["view","control"]},{"name":"id","type":"token","maxLength":12}]}
]}"#;
const URL_PIECES: &[&str] = &[
    "bridge-rdp",
    "BRIDGE-RDP",
    "other",
    ":",
    "//",
    "/",
    "pc0042",
    "10.0.0.1",
    "3389",
    "0",
    "65536",
    "view",
    "control",
    "abc-123",
    "%20",
    "%22",
    "%2F",
    "%5C",
    "%00",
    "%0A",
    "%25",
    "%",
    "%zz",
    " ",
    "\"",
    "'",
    "\\",
    "-",
    "--",
    "/admin",
    "?",
    "#",
    "@",
    "&",
    "|",
    "^",
    ";",
    "..",
    ".",
    "_",
    "ä",
    "\n",
    "\t",
    "\0",
    "%C3%A4",
    "%E2%80%AE",
    "a",
    "Z",
    "9",
];
const ARGUMENT_CHARACTERS: &[char] = &[
    'a', 'b', 'Z', '0', ' ', '\t', '"', '\\', '/', '-', ':', '=', '%', '^', '&', '|', '<', '>',
    'ä', '\'', '(', ')', '!', '*', '?', '.', ',', ';',
];

// Random
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, limit: usize) -> usize {
        usize::try_from(self.next() % limit as u64).unwrap()
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

// Helpers
fn config() -> Config {
    config::parse(CONFIG.as_bytes()).unwrap()
}

fn split(command_line: &str) -> Vec<String> {
    let wide = to_wide(command_line);
    let mut count = 0;
    let arguments = unsafe { CommandLineToArgvW(wide.as_ptr(), &raw mut count) };
    assert!(!arguments.is_null());
    let result = (0..usize::try_from(count).unwrap())
        .map(|index| from_wide_pointer(unsafe { *arguments.add(index) }))
        .collect();
    unsafe {
        LocalFree(arguments.cast());
    }
    result
}

fn random_url(random: &mut Random) -> String {
    let count = 1 + random.below(10);
    (0..count).map(|_| *random.pick(URL_PIECES)).collect()
}

fn likely_url(random: &mut Random) -> String {
    let slots: [&[&str]; 4] = [
        &["pc0042", "10.0.0.1", "PC-1.example.com"],
        &["3389", "1", "65535"],
        &["view", "control"],
        &["abc-123", "A_b", "x"],
    ];
    let mut url = String::from("bridge-rdp://");
    for (index, slot) in slots.iter().enumerate() {
        if index > 0 {
            url.push('/');
        }
        match random.below(8) {
            0 => url.push_str(random.pick(URL_PIECES)),
            1 => {
                url.push_str(random.pick(slot));
                url.push_str(random.pick(URL_PIECES));
            }
            _ => url.push_str(random.pick(slot)),
        }
    }
    if random.below(4) == 0 {
        url.push('/');
    }
    url
}

// Tests
#[test]
fn json_parser_never_panics() {
    let mut random = Random(0x9E37_79B9_7F4A_7C15);
    let pieces: &[&str] = &[
        "{",
        "}",
        "[",
        "]",
        ":",
        ",",
        "\"",
        "\\",
        "\\u",
        "d800",
        "\\n",
        "a",
        "0",
        "1",
        "-",
        ".",
        "e",
        "+",
        "true",
        "false",
        "null",
        " ",
        "\n",
        "\u{feff}",
        "ä",
        "/*",
        "//",
        "\0",
        "\"protocols\"",
    ];
    for _ in 0..ROUNDS {
        let count = random.below(40);
        let text: String = (0..count).map(|_| *random.pick(pieces)).collect();
        let _ = json::parse(text.as_bytes());
        let _ = config::parse(text.as_bytes());
    }
    for _ in 0..ROUNDS {
        let mut bytes = CONFIG.as_bytes().to_vec();
        for _ in 0..=random.below(3) {
            let position = random.below(bytes.len());
            bytes[position] = u8::try_from(random.below(256)).unwrap();
        }
        let _ = config::parse(&bytes);
    }
}

#[test]
fn url_parser_never_panics_and_returns_only_non_empty_values() {
    let mut random = Random(0x1234_5678_9ABC_DEF1);
    for _ in 0..ROUNDS {
        let url = random_url(&mut random);
        if let Ok(request) = url::parse(&url) {
            assert!(
                request.values.iter().all(|value| !value.is_empty()),
                "{url}"
            );
            assert!(
                request
                    .scheme
                    .bytes()
                    .all(|byte| !byte.is_ascii_uppercase())
            );
        }
    }
}

#[test]
fn accepted_urls_only_fill_placeholders_with_safe_values() {
    let config = config();
    let mut random = Random(0x0BAD_C0DE_1234_5678);
    let mut accepted = 0;
    for round in 0..ROUNDS {
        let url = if round % 2 == 0 {
            likely_url(&mut random)
        } else {
            random_url(&mut random)
        };
        let Ok(plan) = Plan::new(&config, &url) else {
            continue;
        };
        accepted += 1;
        assert_eq!(plan.arguments.len(), 6, "{url}");
        let (host, port) = plan.arguments[0]
            .strip_prefix("/v:")
            .and_then(|rest| rest.rsplit_once(':'))
            .unwrap_or_else(|| panic!("{url}"));
        let id = plan.arguments[3].strip_prefix("--id=").unwrap();
        for value in [host, port, plan.arguments[2].as_str(), id] {
            assert!(value::is_safe(value), "{url}");
        }
        assert_eq!(plan.arguments[1], "--mode", "{url}");
        assert!(
            matches!(plan.arguments[2].as_str(), "view" | "control"),
            "{url}"
        );
        assert_eq!(plan.arguments[4], "--title", "{url}");
        assert_eq!(plan.arguments[5], "Remote \"control\" \\", "{url}");
        let mut expected = vec![plan.protocol.program.path.clone()];
        expected.extend(plan.arguments.iter().cloned());
        assert_eq!(split(&plan.command_line()), expected, "{url}");
    }
    assert!(accepted > 1000, "only {accepted} URLs were accepted");
}

#[test]
fn quoting_round_trips_through_windows() {
    let mut random = Random(0xFEED_FACE_CAFE_BEEF);
    for _ in 0..ROUNDS {
        let arguments: Vec<String> = (0..=random.below(4))
            .map(|_| {
                (0..random.below(12))
                    .map(|_| *random.pick(ARGUMENT_CHARACTERS))
                    .collect()
            })
            .collect();
        let mut line = quote(r"C:\Program Files\Tool\tool.exe");
        for argument in &arguments {
            line.push(' ');
            line.push_str(&quote(argument));
        }
        assert_eq!(split(&line)[1..], arguments, "{line}");
    }
}
