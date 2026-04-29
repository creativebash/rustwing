import os, json

template_dir = "template"
files = []
for root, dirs, filenames in os.walk(template_dir):
    for f in sorted(filenames):
        path = os.path.join(root, f)
        rel = os.path.relpath(path, template_dir)
        content = open(path, "r").read()
        files.append((rel, content))

with open("src/template_data.rs", "w") as out:
    out.write("// Auto-generated. Do not edit manually.\n")
    out.write("// Regenerate: cd cli && python3 gen_template.py\n\n")
    out.write("pub static FILES: &[(&str, &str)] = &[\n")
    for rel, content in files:
        escaped = json.dumps(content, ensure_ascii=False)
        out.write(f'    ({json.dumps(rel)}, {escaped}),\n')
    out.write("];\n")

print(f"Generated {len(files)} template entries in src/template_data.rs")
