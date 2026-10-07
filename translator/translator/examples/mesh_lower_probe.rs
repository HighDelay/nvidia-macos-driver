use metal2vulkan::mesh_lower::*;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let ll = std::fs::read_to_string(&a[1]).unwrap();
    let mode = a.get(4).map(|s| s.as_str());
    let oll = a
        .get(5)
        .map(|p| std::fs::read_to_string(p).unwrap())
        .unwrap_or_else(|| ll.clone());
    let (link, rs) = match mode {
        None => (Link::Direct, 0),
        Some("--indirect") => (Link::Indirect, 16),
        Some(o) => (
            Link::Object,
            record_stride(payload_len(&oll, o).unwrap_or_else(|e| {
                eprintln!("REFUSED: {e}");
                std::process::exit(2)
            })),
        ),
    };
    let cap = if link == Link::Direct { 0 } else { 1024 };
    match lower_mesh_linked(&ll, &a[2], link, rs, cap) {
        Ok(m) => {
            std::fs::write(format!("{}.kernel.ll", a[3]), &m.kernel_ll).unwrap();
            std::fs::write(format!("{}.vertex.ll", a[3]), &m.vertex_ll).unwrap();
            if link == Link::Object {
                match lower_object(&oll, mode.unwrap(), &m.layout, m.tr, rs) {
                    Ok(o) => std::fs::write(format!("{}.object.ll", a[3]), o).unwrap(),
                    Err(e) => {
                        eprintln!("REFUSED object: {e}");
                        std::process::exit(2)
                    }
                }
            }
            println!("{:?} tr {} rs {}", m.layout, m.tr, rs);
        }
        Err(e) => {
            eprintln!("REFUSED: {e}");
            std::process::exit(2)
        }
    }
}
