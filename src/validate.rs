use std::collections::HashSet;

use crate::ir::{BlockItemInner, IR};

#[derive(Debug, Clone)]
pub struct Options {
    pub allow_register_overlap: bool,
    pub allow_field_overlap: bool,
    pub allow_enum_dup_value: bool,
    pub allow_unused_enums: bool,
    pub allow_unused_fieldsets: bool,
}

pub fn validate(ir: &IR, options: Options) -> Vec<String> {
    let mut errs = Vec::new();

    let mut used_fieldsets = HashSet::new();
    let mut used_enums = HashSet::new();

    for (bname, b) in &ir.blocks {
        if let Some(n) = &b.extends {
            if !ir.blocks.contains_key(n) {
                errs.push(format!(
                    "block {}: extends block {} does not exist",
                    bname, n
                ))
            }
        }

        for bi in &b.items {
            match &bi.inner {
                BlockItemInner::Block(i) => {
                    if !ir.blocks.contains_key(&i.block) {
                        errs.push(format!(
                            "block {} item {}: block {} does not exist",
                            bname, bi.name, i.block
                        ))
                    }
                }
                BlockItemInner::Register(i) => {
                    if let Some(fs) = &i.fieldset {
                        used_fieldsets.insert(fs.clone());
                        if !ir.fieldsets.contains_key(fs) {
                            errs.push(format!(
                                "block {} item {}: fieldset {} does not exist",
                                bname, bi.name, fs
                            ))
                        }
                    }
                }
            }
        }

        if !options.allow_register_overlap {
            for (i1, i2) in Pairs::new(b.items.iter()) {
                if i1.byte_offset == i2.byte_offset {
                    errs.push(format!(
                        "block {}: registers overlap: {} {}",
                        bname, i1.name, i2.name
                    ));
                }
            }
        }
    }

    for (fsname, fs) in &ir.fieldsets {
        if !options.allow_unused_fieldsets && !used_fieldsets.contains(fsname) {
            errs.push(format!("fieldset {} is unused", fsname));
        }

        if let Some(n) = &fs.extends {
            if !ir.fieldsets.contains_key(n) {
                errs.push(format!(
                    "fieldset {}: extends fieldset {} does not exist",
                    fsname, n
                ))
            }
        }

        for f in &fs.fields {
            if let Some(ename) = &f.enumm {
                used_enums.insert(ename.clone());

                let Some(e) = ir.enums.get(ename) else {
                    errs.push(format!(
                        "fieldset {} field {}: enum {} does not exist",
                        fsname, f.name, ename
                    ));
                    continue
                };
                if f.bit_size != e.bit_size {
                    errs.push(format!(
                        "fieldset {} field {}: bit_size {} does not match enum {} bit_size {}",
                        fsname, f.name, f.bit_size, ename, e.bit_size
                    ));
                }
            }
        }

        if !options.allow_field_overlap {
            for (i1, i2) in Pairs::new(fs.fields.iter()) {
                if i2.bit_offset + i2.bit_size > i1.bit_offset
                    && i1.bit_offset + i1.bit_size > i2.bit_offset
                {
                    errs.push(format!(
                        "fieldset {}: fields overlap: {} {}",
                        fsname, i1.name, i2.name
                    ));
                }
            }
        }
    }

    for (ename, e) in &ir.enums {
        if !options.allow_unused_enums && !used_enums.contains(ename) {
            errs.push(format!("enum {} is unused", ename));
        }

        let maxval = 1 << e.bit_size;
        for v in &e.variants {
            if v.value >= maxval {
                errs.push(format!(
                    "enum {} variant {}: value {} is not less than than max 1<<{} = {}",
                    ename, v.name, v.value, e.bit_size, maxval,
                ));
            }
        }

        if !options.allow_enum_dup_value {
            for (i1, i2) in Pairs::new(e.variants.iter()) {
                if i1.value == i2.value {
                    errs.push(format!(
                        "enum {}: variants with same value: {} {}",
                        ename, i1.name, i2.name
                    ));
                }
            }
        }
    }

    errs
}

// ==============

struct Pairs<U: Iterator + Clone> {
    head: Option<U::Item>,
    tail: U,
    next: U,
}

impl<U: Iterator + Clone> Pairs<U> {
    fn new(mut iter: U) -> Self {
        let head = iter.next();
        Pairs {
            head,
            tail: iter.clone(),
            next: iter,
        }
    }
}

impl<U: Iterator + Clone> Iterator for Pairs<U>
where
    U::Item: Clone,
{
    type Item = (U::Item, U::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let a = self.head.as_ref()?.clone();

        if let Some(b) = self.tail.next() {
            return Some((a, b));
        }

        match self.next.next() {
            Some(new_head) => {
                self.head = Some(new_head);
                self.tail = self.next.clone();
                self.next()
            }
            None => None,
        }
    }
}
