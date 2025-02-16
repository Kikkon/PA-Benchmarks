use arrow2::array::*;
use arrow2::chunk::Chunk;
use arrow2::datatypes::Field;
use arrow2::datatypes::Schema;
use arrow2::error::Result;
use arrow2::io::parquet::write::*;
use criterion::*;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

fn create_array(size: usize, ty: &str) -> Box<dyn Array> {
    let i64 = [
        Some(0),
        Some(1),
        None,
        Some(3),
        Some(4),
        Some(5),
        Some(6),
        Some(7),
    ];

    let utf8 = [
        Some("aaaa"),
        Some("aaab"),
        None,
        Some("aaac"),
        Some("aaad"),
        Some("aaae"),
        Some("aaaf"),
        Some("aaag"),
    ];

    let bool = [
        Some(true),
        Some(false),
        None,
        Some(true),
        Some(false),
        Some(true),
        Some(true),
        Some(true),
    ];

    let array = match ty {
        "i64" => Arc::new(i64.iter().cycle().take(size).collect::<Int64Array>()) as Arc<dyn Array>,
        "utf8" => Arc::new(
            utf8.iter()
                .cloned()
                .cycle()
                .take(size)
                .collect::<Utf8Array<i32>>(),
        ) as Arc<dyn Array>,
        "bool" => {
            Arc::new(bool.iter().cycle().take(size).collect::<BooleanArray>()) as Arc<dyn Array>
        }
        _ => todo!(),
    };
    array.to_boxed()
}

fn write_chunk(path: &PathBuf, array: &Box<dyn Array>, is_compressed: bool) -> Result<()> {
    let file = File::create(path)?;
    let chunk = Chunk::new(vec![array.to_boxed()]);
    let filed = Field::new("column", array.data_type().clone(), true);
    let schema = Schema::from(vec![filed]);
    let compression = if is_compressed {
        CompressionOptions::Snappy
    } else {
        CompressionOptions::Uncompressed
    };
    let options = WriteOptions {
        write_statistics: true,
        compression: compression,
        version: Version::V2,
        data_pagesize_limit: None,
    };

    let iter = vec![Ok(chunk)];

    let encodings = schema
        .fields
        .iter()
        .map(|f| transverse(&f.data_type, |_| Encoding::Plain))
        .collect();

    let row_groups = RowGroupIterator::try_new(iter.into_iter(), &schema, options, encodings)?;

    let mut writer = FileWriter::try_new(file, schema, options)?;
    for group in row_groups {
        writer.write(group?)?;
    }
    let _size = writer.end(None)?;
    Ok(())
}

fn add_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");

    for log2_size in (10..=20).step_by(2) {
        let size = 2usize.pow(log2_size);
        group.throughput(Throughput::Elements(size as u64));
        for ty in ["i64", "utf8", "bool"] {
            let array = create_array(size, ty);
            for is_compressed in [true, false] {
                let id = if is_compressed {
                    format!("{} snappy", ty)
                } else {
                    ty.to_string()
                };
                let dir = env!("CARGO_MANIFEST_DIR");
                let path = PathBuf::from(dir).join(format!(
                    "fixtures/pyarrow/v1{}benches_{}.parquet",
                    is_compressed, size
                ));

                group.bench_with_input(BenchmarkId::new(id, log2_size), &path, |b, path| {
                    b.iter(|| write_chunk(&path, &array, is_compressed).unwrap())
                });
            }
        }
    }
}

criterion_group!(benches, add_benchmark);
criterion_main!(benches);
