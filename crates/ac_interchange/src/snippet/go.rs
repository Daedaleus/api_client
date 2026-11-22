//! Go with `net/http`.

use std::fmt::Write as _;

use ac_domain::exchange::{PartContent, RequestBody, ResolvedRequest};

use super::{literal, text_of};

/// A `main` package that sends the request and prints the response.
pub(super) fn net_http(request: &ResolvedRequest) -> String {
    let mut imports = vec!["\"fmt\"", "\"io\"", "\"net/http\"", "\"time\""];
    let mut setup = String::new();
    let body = match &request.body {
        None => "nil".to_owned(),
        Some(RequestBody::Bytes(bytes)) => {
            imports.push("\"strings\"");
            format!("strings.NewReader({})", literal(&text_of(bytes)))
        }
        Some(RequestBody::File(path)) => {
            imports.push("\"os\"");
            let _ = writeln!(
                setup,
                "\tfile, err := os.Open({})\n\tif err != nil {{\n\t\tpanic(err)\n\t}}\n\tdefer file.Close()\n",
                literal(&path.display().to_string())
            );
            "file".to_owned()
        }
        Some(RequestBody::Multipart(parts)) => {
            imports.extend(["\"bytes\"", "\"mime/multipart\""]);
            setup.push_str("\tvar body bytes.Buffer\n\tform := multipart.NewWriter(&body)\n");
            for part in parts {
                match &part.content {
                    PartContent::Text(text) => {
                        let _ = writeln!(
                            setup,
                            "\tform.WriteField({}, {})",
                            literal(&part.name),
                            literal(text)
                        );
                    }
                    PartContent::File(path) => {
                        if !imports.contains(&"\"os\"") {
                            imports.push("\"os\"");
                        }
                        let name = path
                            .file_name()
                            .map_or_default(|name| name.to_string_lossy().into_owned());
                        let _ = writeln!(
                            setup,
                            "\tif data, err := os.ReadFile({}); err == nil {{\n\t\tif part, err := form.CreateFormFile({}, {}); err == nil {{\n\t\t\tpart.Write(data)\n\t\t}}\n\t}}",
                            literal(&path.display().to_string()),
                            literal(&part.name),
                            literal(&name)
                        );
                    }
                }
            }
            setup.push_str("\tform.Close()\n\n");
            "&body".to_owned()
        }
    };
    if !request.verify_tls {
        imports.push("\"crypto/tls\"");
    }
    imports.sort_unstable();

    let mut code = String::from("package main\n\nimport (\n");
    for import in &imports {
        let _ = writeln!(code, "\t{import}");
    }
    code.push_str(")\n\nfunc main() {\n");
    code.push_str(&setup);
    let _ = writeln!(
        code,
        "\treq, err := http.NewRequest({}, {}, {body})\n\tif err != nil {{\n\t\tpanic(err)\n\t}}",
        literal(request.method.as_str()),
        literal(request.url.as_str())
    );
    for header in &request.headers {
        let _ = writeln!(
            code,
            "\treq.Header.Set({}, {})",
            literal(&header.name),
            literal(&header.value)
        );
    }
    if matches!(request.body, Some(RequestBody::Multipart(_))) {
        code.push_str("\treq.Header.Set(\"Content-Type\", form.FormDataContentType())\n");
    }
    let _ = writeln!(
        code,
        "\n\tclient := &http.Client{{Timeout: {} * time.Second}}",
        request.transport.timeout.as_secs().max(1)
    );
    if !request.transport.follow_redirects {
        code.push_str(
            "\tclient.CheckRedirect = func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }\n",
        );
    }
    if !request.verify_tls {
        code.push_str(
            "\t// Certificate check off (test servers only).\n\tclient.Transport = &http.Transport{TLSClientConfig: &tls.Config{InsecureSkipVerify: true}}\n",
        );
    }
    code.push_str(
        "\tresponse, err := client.Do(req)\n\tif err != nil {\n\t\tpanic(err)\n\t}\n\tdefer response.Body.Close()\n\tdata, _ := io.ReadAll(response.Body)\n\tfmt.Println(response.Status, string(data))\n}\n",
    );
    code
}
