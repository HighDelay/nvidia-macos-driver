fn main() {
    let args: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&args[1]).expect("read");
    let out = std::path::PathBuf::from(&args[2]);
    let mut values = Vec::new();
    for spec in args[3..].iter() {
        let (i, v) = spec.split_once('=').expect("index=hexbytes");
        let bytes: Vec<u8> = v
            .as_bytes()
            .chunks(2)
            .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
            .collect();
        values.push((i.parse::<u32>().unwrap(), bytes));
    }
    let tmp = std::env::temp_dir().join(format!("fcspec-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).ok();
    let options = metal2vulkan::passes::TransformOptions::default();
    match metal2vulkan::translate_sanitized_native_specialized_reflected_with_options(
        &src,
        metal2vulkan::passes::Stage::Kernel,
        &tmp,
        options,
        &values,
    ) {
        Ok((spv, _)) => {
            std::fs::write(&out, &spv).expect("write");
            println!("OK {} bytes", spv.len());
        }
        Err(e) => println!("ERR {e}"),
    }
    std::fs::remove_dir_all(&tmp).ok();
}
