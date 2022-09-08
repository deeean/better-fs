use std::fs::read_dir;
use std::time::Instant;
use async_std::path::Path;
use async_std::prelude::*;
use async_std::task;
use async_std::future;
use futures::join;
use futures::future::join_all;
use async_recursion::async_recursion;

#[derive(Debug)]
pub struct Entity {
  pub file_name: String,
  pub path: async_std::path::PathBuf,
  pub metadata: async_std::fs::Metadata,
}

impl Entity {
  pub fn new(file_name: String, path: async_std::path::PathBuf, metadata: async_std::fs::Metadata) -> Self {
    Self {
      file_name,
      path,
      metadata,
    }
  }
}

pub struct Fs;

impl Fs {
  pub async fn read_dir<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<Entity>> {
    let mut dir = async_std::fs::read_dir(path).await?;
    let mut entries = Vec::new();

    while let Some(res) = dir.next().await {
      let entry = res?;
      entries.push(entry);
    }

    let metadata = join_all(
      entries
        .iter()
        .map(|it|
          it.metadata()
        )).await
      .into_iter()
      .collect::<Result<Vec<async_std::fs::Metadata>, std::io::Error>>();

    match metadata {
      Ok(metadata) => {
        let entities = entries
          .into_iter()
          .zip(metadata)
          .map(|(entry, metadata)| {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            Entity::new(file_name, path, metadata)
          })
          .collect::<Vec<_>>();

        Ok(entities)
      }
      Err(e) => Err(e),
    }
  }

  #[async_recursion(?Send)]
  pub async fn read_dir_recursive<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<Entity>> {
    let mut entities = Self::read_dir(path).await?;

    let sub_dirs = entities
      .iter()
      .filter(|it| it.metadata.is_dir())
      .map(|it| it.path.clone())
      .collect::<Vec<_>>();

    let sub_entities = join_all(
      sub_dirs
        .into_iter()
        .map(|it| Self::read_dir_recursive(it))).await
      .into_iter()
      .collect::<Result<Vec<Vec<Entity>>, std::io::Error>>();

    match sub_entities {
      Ok(mut sub_entities) => {
        entities.append(&mut sub_entities.into_iter().flatten().collect::<Vec<_>>());
        Ok(entities)
      }
      Err(e) => Err(e),
    }
  }
}

async fn bootstrap() -> std::io::Result<()> {
  let now = Instant::now();
  let entities = Fs::read_dir_recursive("/Users/dean/Documents/web-frameworks").await?;
  println!("Elapsed: {:?}", now.elapsed());
  println!("{:?}", entities.len());

  Ok(())
}

fn main() {
  task::block_on(bootstrap());
}
