/*
 * pacwrap
 *
 * Copyright (C) 2023-2024 Xavier Moffett <sapphirus@azorium.net>
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, version 3.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::{
    collections::HashMap,
    fs::read_dir,
    hash::{Hash, Hasher},
    os::unix::fs::MetadataExt,
};

use indexmap::IndexSet;
use simplebyteunit::simplebyteunit::*;

use pacwrap_core::{
    config::{cache::populate, ContainerType},
    constants::{BOLD, CONTAINER_DIR, RESET, UNDERLINE},
    utils::{
        arguments::Operand,
        table::{ColumnAttribute, Table},
        whitespace,
        Arguments,
    },
    ErrorGeneric,
    Result,
};

use Display::*;

#[derive(Eq)]
enum Display {
    Total(bool),
    Organic(bool),
    Summary(Option<bool>),
    Name,
    Type,
}

impl Hash for Display {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(match self {
            Self::Name => 0,
            Self::Type => 1,
            Self::Summary(_) => 2,
            Self::Total(bytes) => 3 + *bytes as u8,
            Self::Organic(bytes) => 5 + *bytes as u8,
        })
    }
}

impl PartialEq for Display {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Summary(_), Self::Summary(_))
                | (Self::Total(_), Self::Total(_))
                | (Self::Organic(_), Self::Organic(_))
                | (Self::Name, Self::Name)
                | (Self::Type, Self::Type)
        )
    }
}

impl Display {
    fn bytes(&self) -> bool {
        match self {
            Self::Summary(bytes) => bytes.unwrap_or_default(),
            Self::Total(bytes) => *bytes,
            Self::Organic(bytes) => *bytes,
            _ => false,
        }
    }
}

fn parse_arguments(args: &mut Arguments) -> Result<(bool, IndexSet<Display>)> {
    let mut bytes = false;
    let mut vec = vec![Name, Type];

    while let Some(arg) = args.next() {
        match arg {
            Operand::Short('b') | Operand::Long("bytes") => bytes = !bytes,
            Operand::Short('s') | Operand::Long("summary") => vec.push(Summary(Some(bytes))),
            Operand::Short('t') | Operand::Long("total") => vec.push(Total(bytes)),
            Operand::Short('o') | Operand::Long("on-disk") => vec.push(Organic(bytes)),
            _ => args.invalid_operand()?,
        }
    }

    Ok((vec.len() > 2, IndexSet::from_iter(vec)))
}

pub fn list_containers(args: &mut Arguments) -> Result<()> {
    let handles = populate()?;
    let mut handles = handles.registered_handles();
    let (measure_disk, table_type) = parse_arguments(args)?;
    let containers = &format!("Containers ({})", handles.len());
    let mut container_sizes: HashMap<&str, (i64, i64)> = HashMap::new();
    let mut actual_size = 0;
    let mut total_size = 0;
    let mut table_header: Vec<&str> = vec![];

    for column in &table_type {
        match column {
            Name => table_header.push(containers),
            Type => table_header.push("Type"),
            Total(_) => table_header.push("Total"),
            Organic(_) => table_header.push("Size on Disk"),
            _ => continue,
        }
    }

    let mut table = Table::new().header(&table_header).spacing(4);

    for col in 2 .. if table_type.contains(&Summary(None)) {
        table_type.len() - 1
    } else {
        table_type.len()
    } {
        table = table.col_attribute(col, ColumnAttribute::AlignRight);
    }

    //TODO: More advanced sorting options
    handles.sort_by_key(|f| *f.metadata().container_type() == ContainerType::Base);
    handles.sort_by_key(|f| *f.metadata().container_type() == ContainerType::Slice);
    handles.sort_by_key(|f| *f.metadata().container_type() == ContainerType::Aggregate);
    handles.sort_by_key(|f| *f.metadata().container_type() == ContainerType::Symbolic);

    for container in handles.iter() {
        let instance = container.vars().instance();
        let container_path = &format!("{}/{}", *CONTAINER_DIR, instance);
        let (len, organic, total) = if measure_disk && container.metadata().container_type() != &ContainerType::Symbolic {
            directory_size(container_path)?
        } else {
            (0, 0, 0)
        };

        total_size += total;
        actual_size += len + organic;
        container_sizes.insert(instance, (len + organic, total));
    }

    for container in handles {
        let container_name = container.vars().instance();
        let container_type = container.metadata().container_type();
        let (organic, total) = container_sizes.get(container_name).unwrap();
        let mut row = vec![];

        for column in &table_type {
            match column {
                Name => row.push(container_name.to_string()),
                Type => row.push(container_type.to_string()),
                Total(bytes) => row.push(match bytes {
                    false => total.to_byteunit(SI).to_string(),
                    true => total.to_string(),
                }),
                Organic(bytes) => row.push(match bytes {
                    false => organic.to_byteunit(SI).to_string(),
                    true => organic.to_string(),
                }),
                _ => continue,
            }
        }

        table.insert(row);
    }

    if let Some(sum) = table_type.get(&Display::Summary(None)) {
        let mut max_len = 0;
        let difference = total_size - actual_size;
        let equation = match sum.bytes() {
            true => vec![total_size.to_string(), difference.to_string(), actual_size.to_string()],
            false => vec![
                total_size.to_byteunit(SI).to_string(),
                difference.to_byteunit(SI).to_string(),
                actual_size.to_byteunit(SI).to_string(),
            ],
        };

        for eq in &equation {
            if eq.len() > max_len {
                max_len = eq.len();
            }
        }

        println!(
            "{}\nTotal Size:      {}{} \nDifference:   {}{} - {} {} \n{}Size on Disk{}:    {}{}\n",
            table.build()?,
            whitespace(max_len - equation[0].len()),
            equation[0],
            *UNDERLINE,
            whitespace(max_len - equation[1].len()),
            equation[1],
            *RESET,
            *BOLD,
            *RESET,
            whitespace(max_len - equation[2].len()),
            equation[2]
        )
    } else {
        println!("{}", table.build()?)
    };
    Ok(())
}

//There might be some value in threading this routine in future.
fn directory_size(dir: &str) -> Result<(i64, i64, i64)> {
    let mut len = 0;
    let mut total = 0;
    let mut unique = 0;

    for entry in read_dir(dir).prepend_io(|| dir.into())? {
        let entry = entry.prepend(|| format!("Failure acquiring entry in '{dir}'"))?;
        let name = entry.file_name().to_str().unwrap().to_string();
        let meta = entry.metadata().prepend(|| format!("Failure to acquire metadata in '{dir}/{name}'"))?;

        if entry
            .file_type()
            .prepend(|| format!("Failure to acquire filetype '{dir}/{name}'"))?
            .is_dir()
        {
            let (l, u, t) = directory_size(&format!("{dir}/{name}"))?;

            len += l;
            unique += u;
            total += t;
        } else if meta.nlink() == 1 {
            unique += meta.len() as i64;
        } else {
            len += (meta.len() / meta.nlink()) as i64;
            total += meta.len() as i64;
        }
    }

    Ok((len, unique, total))
}
