//! format.*, integration.embedding

pub(crate) mod util {
    use crate::common::*;
    use std::path::Path;

    /// Open `p` in a fresh app; returns the app.
    pub fn opened(p: &Path) -> HResult<App> {
        let mut a = start()?;
        open_path(&mut a, p)?;
        Ok(a)
    }

    /// Open `p` and expect the cannot-open message; previous image kept.
    pub fn expect_cannot_open(p: &Path) -> T {
        let mut a = fresh()?;
        a.set_pixel(0, 0, RED)?;
        open_path(&mut a, p)?;
        let d = expect_dialog(&a, DialogKind::MessageBox)?;
        assert!(d.text.contains("This file cannot be opened"), "got: {}", d.text);
        a.dialog_click("OK")?;
        assert_eq!(size(&a)?, (32, 32));
        assert_eq!(px(&a, 0, 0)?, RED);
        Ok(())
    }

    pub const PNG: &[u8] = &[
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0, 1, 0, 0, 0, 1,
        8, 2, 0, 0, 0, 0x90, 0x77, 0x53, 0xDE, 0, 0, 0, 12, b'I', b'D', b'A', b'T', 0x08, 0xD7, 0x63, 0xF8, 0xCF,
        0xC0, 0, 0, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE,
        0x42, 0x60, 0x82,
    ];

    /// Save the document with Save As at type `ty`, accepting any warning,
    /// and return the written bytes.
    pub fn saved_bytes(a: &mut App, ty: &str) -> HResult<Vec<u8>> {
        let p = tmp("w").join("out.bmp");
        save_as_ok(a, &p, ty)?;
        read(&p)
    }
}

mod format_bmp_read {
    use super::util::*;
    use crate::common::*;

    #[test]
    fn rgbw_24bit() -> T {
        let p = tmp("r").join("rgbw.bmp");
        write(&p, &bmp24(2, 2, &[RED, GREEN, BLUE, W], false))?;
        let a = opened(&p)?;
        assert_eq!(size(&a)?, (2, 2));
        assert_eq!((px(&a, 0, 0)?, px(&a, 1, 0)?, px(&a, 0, 1)?, px(&a, 1, 1)?), (RED, GREEN, BLUE, W));
        Ok(())
    }

    #[test]
    fn top_down() -> T {
        let p = tmp("r").join("td.bmp");
        write(&p, &bmp24(2, 2, &[RED, GREEN, BLUE, W], true))?;
        let a = opened(&p)?;
        assert_eq!(a.pixels()?.pixels, [RED, GREEN, BLUE, W]);
        Ok(())
    }

    #[test]
    fn rle8_matches_raw() -> T {
        let dir = tmp("rle");
        let pal = [K, RED, GREEN, BLUE];
        let raw = dir.join("raw.bmp");
        write(&raw, &bmp_indexed(4, 2, 8, &pal, &[1, 1, 1, 2, 3, 3, 0, 0]))?;
        let rle = dir.join("rle.bmp");
        // bottom row [3,3,0,0], then top row [1,1,1,2], end of bitmap.
        let data = [2, 3, 2, 0, 0, 0, 3, 1, 1, 2, 0, 0, 0, 1];
        write(&rle, &bmp_raw(4, 2, 8, 1, &pal, &[], &data))?;
        let a = opened(&raw)?;
        let b = opened(&rle)?;
        assert_eq!(b.pixels()?, a.pixels()?);
        assert_eq!(b.pixels()?.pixels, [RED, RED, RED, GREEN, BLUE, BLUE, K, K]);
        Ok(())
    }

    #[test]
    fn png_rejected() -> T {
        let p = tmp("png").join("x.bmp");
        write(&p, PNG)?;
        expect_cannot_open(&p)
    }

    #[test]
    fn pcx_rejected() -> T {
        let p = tmp("pcx").join("x.pcx");
        let mut b = vec![0x0A, 0x05, 0x01, 0x08, 0, 0, 0, 0, 1, 0, 1, 0, 72, 0, 72, 0];
        b.resize(128 + 8, 0);
        write(&p, &b)?;
        expect_cannot_open(&p)
    }

    #[test]
    fn eight_bit_keeps_palette() -> T {
        let p = tmp("r8").join("p.bmp");
        let pal = [rgb(10, 20, 30), rgb(40, 50, 60), rgb(70, 80, 90)];
        write(&p, &bmp_indexed(2, 1, 8, &pal, &[2, 0]))?;
        let a = opened(&p)?;
        assert_eq!(a.pixels()?.pixels, [pal[2], pal[0]]);
        let d = a.document()?;
        assert_eq!(d.source_depth, 8);
        let got = d.palette.unwrap_or_default();
        assert!(got.len() >= 3 && got[..3] == pal, "palette {got:?}");
        Ok(())
    }

    #[test]
    fn four_bit_pixels() -> T {
        let p = tmp("r4").join("p.bmp");
        write(&p, &bmp_indexed(3, 2, 4, &PAL16, &[9, 10, 12, 0, 15, 11]))?;
        let a = opened(&p)?;
        assert_eq!(a.pixels()?.pixels, [PAL16[9], PAL16[10], PAL16[12], PAL16[0], PAL16[15], PAL16[11]]);
        assert_eq!(a.document()?.source_depth, 4);
        Ok(())
    }

    #[test]
    fn one_bit_pixels() -> T {
        let p = tmp("r1").join("p.bmp");
        let pal = [rgb(255, 0, 0), rgb(0, 0, 255)];
        let idx: Vec<u8> = (0..18).map(|i| ((i * 7) % 3 == 0) as u8).collect();
        write(&p, &bmp_indexed(9, 2, 1, &pal, &idx))?;
        let a = opened(&p)?;
        let want: Vec<Rgb> = idx.iter().map(|&i| pal[i as usize]).collect();
        assert_eq!(a.pixels()?.pixels, want);
        assert_eq!(a.document()?.source_depth, 1);
        Ok(())
    }

    #[test]
    fn sixteen_bit_555() -> T {
        let p = tmp("r16").join("p.bmp");
        // red 31, blue 31, green 16 (5-5-5), row padded to 8 bytes.
        let data = [0x00, 0x7C, 0x1F, 0x00, 0x00, 0x02, 0, 0];
        write(&p, &bmp_raw(3, 1, 16, 0, &[], &[], &data))?;
        let a = opened(&p)?;
        let v = a.pixels()?.pixels;
        let full = |c: u8| c == 248 || c == 255;
        assert!(full(v[0].r) && v[0].g == 0 && v[0].b == 0, "{:?}", v[0]);
        assert!(v[1].r == 0 && v[1].g == 0 && full(v[1].b), "{:?}", v[1]);
        assert!(v[2].r == 0 && (v[2].g == 128 || v[2].g == 132) && v[2].b == 0, "{:?}", v[2]);
        assert_eq!(a.document()?.source_depth, 24);
        assert_eq!(a.document()?.palette, None);
        Ok(())
    }

    #[test]
    fn core_header_24bit() -> T {
        let p = tmp("core").join("c.bmp");
        let mut b = Vec::new();
        b.extend_from_slice(b"BM");
        b.extend_from_slice(&(26u32 + 8).to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&26u32.to_le_bytes());
        b.extend_from_slice(&12u32.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&24u16.to_le_bytes());
        b.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        write(&p, &b)?;
        let a = opened(&p)?;
        assert_eq!(a.pixels()?.pixels, [RED, GREEN]);
        Ok(())
    }
}

mod format_bmp_write {
    use super::util::*;
    use crate::common::*;

    #[test]
    fn two_by_two_header() -> T {
        let mut a = image(2, 2, &[RED, GREEN, BLUE, W])?;
        let b = saved_bytes(&mut a, T24)?;
        assert_eq!(b.len(), 70);
        assert_eq!(&b[0..2], &[0x42, 0x4D]);
        assert_eq!(le32(&b, 2), 70);
        assert_eq!(le32(&b, 10), 54);
        assert_eq!(le32(&b, 18), 2);
        assert_eq!(le32(&b, 22), 2);
        assert_eq!(le16(&b, 28), 24);
        assert_eq!(le32(&b, 30), 0);
        assert_eq!(le32(&b, 38), 3780);
        Ok(())
    }

    #[test]
    fn two_by_two_pixel_layout() -> T {
        let mut a = image(2, 2, &[RED, GREEN, BLUE, rgb(1, 2, 3)])?;
        let b = saved_bytes(&mut a, T24)?;
        assert_eq!(&b[54..57], &[255, 0, 0]); // pixel (0,1) = BLUE as B,G,R
        assert_eq!(&b[57..60], &[3, 2, 1]);
        assert_eq!(&b[60..62], &[0, 0]);
        assert_eq!(&b[62..68], &[0, 0, 255, 0, 255, 0]);
        Ok(())
    }

    #[test]
    fn mono_9x1() -> T {
        let px: Vec<Rgb> = (0..9).map(|i| if i % 2 == 0 { K } else { W }).collect();
        let mut a = image(9, 1, &px)?;
        let b = saved_bytes(&mut a, T1)?;
        assert_eq!(b.len(), 66);
        assert_eq!(le32(&b, 10), 62);
        assert_eq!(&b[54..62], &[0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0]);
        assert_eq!(&b[62..66], &[0x55, 0x00, 0, 0]);
        Ok(())
    }

    #[test]
    fn eight_bit_palette_kept() -> T {
        let p = tmp("w8").join("five.bmp");
        let pal = [rgb(1, 2, 3), rgb(200, 10, 10), rgb(9, 99, 199), rgb(33, 66, 99), rgb(250, 250, 5)];
        write(&p, &bmp_indexed(3, 1, 8, &pal, &[0, 3, 4]))?;
        let mut a = opened(&p)?;
        set_fg(&mut a, pal[1])?;
        pencil_click(&mut a, 0, 0)?;
        a.menu("File > Save")?;
        let b = read(&p)?;
        assert_eq!(le16(&b, 28), 8);
        let off = 14 + le32(&b, 14) as usize;
        for (i, c) in pal.iter().enumerate() {
            assert_eq!(&b[off + 4 * i..off + 4 * i + 4], &[c.b, c.g, c.r, 0], "entry {i}");
        }
        assert_eq!(decode(&b)?.image.pixels, [pal[1], pal[3], pal[4]]);
        Ok(())
    }

    #[test]
    fn four_bit_layout() -> T {
        let mut a = image(3, 1, &[RED, rgb(0, 128, 128), W])?;
        let b = saved_bytes(&mut a, T4)?;
        assert_eq!(b.len(), 122);
        assert_eq!(le32(&b, 10), 118);
        assert_eq!(le16(&b, 28), 4);
        assert_eq!(le32(&b, 46), 16);
        for (i, c) in PAL16.iter().enumerate() {
            assert_eq!(&b[54 + 4 * i..58 + 4 * i], &[c.b, c.g, c.r, 0], "entry {i}");
        }
        assert_eq!(&b[118..122], &[0x96, 0xF0, 0, 0]);
        Ok(())
    }

    #[test]
    fn header_constant_fields() -> T {
        let mut a = image(3, 2, &pattern(3, 2, 4))?;
        let b = saved_bytes(&mut a, T24)?;
        assert_eq!(b.len(), 54 + 24);
        assert_eq!(le32(&b, 6), 0);
        assert_eq!(le32(&b, 14), 40);
        assert_eq!(le32s(&b, 22), 2);
        assert_eq!(le16(&b, 26), 1);
        assert_eq!(le32(&b, 30), 0);
        assert_eq!(le32(&b, 34), 24);
        assert_eq!(le32(&b, 38), 3780);
        assert_eq!(le32(&b, 42), 3780);
        assert_eq!(le32(&b, 46), 0);
        assert_eq!(le32(&b, 50), 0);
        assert_eq!(decode(&b)?.image.pixels, pattern(3, 2, 4));
        Ok(())
    }
}

mod format_color_reduction {
    use super::util::*;
    use crate::common::*;

    pub fn pal256() -> Vec<Rgb> {
        let mut v = PAL16.to_vec();
        for r in 0..6u8 {
            for g in 0..6u8 {
                for b in 0..6u8 {
                    v.push(rgb(51 * r, 51 * g, 51 * b));
                }
            }
        }
        for k in 0..24u8 {
            let c = 8 + 10 * k;
            v.push(rgb(c, c, c));
        }
        v
    }

    #[test]
    fn mono_threshold() -> T {
        let mut a = image(2, 1, &[rgb(200, 100, 50), rgb(128, 128, 128)])?;
        let b = saved_bytes(&mut a, T1)?;
        assert_eq!(b[62], 0x40);
        Ok(())
    }

    #[test]
    fn four_bit_nearest() -> T {
        let mut a = image(1, 1, &[rgb(250, 10, 10)])?;
        let b = saved_bytes(&mut a, T4)?;
        assert_eq!(b[118] >> 4, 9);
        assert_eq!(decode(&b)?.image.get(0, 0), Some(RED));
        Ok(())
    }

    #[test]
    fn eight_bit_tie_lower_index() -> T {
        let mut a = image(1, 1, &[rgb(100, 100, 100)])?;
        let b = saved_bytes(&mut a, T8)?;
        assert_eq!(le32(&b, 10), 1078);
        assert_eq!(b[1078], 102);
        assert_eq!(decode(&b)?.palette, pal256());
        Ok(())
    }

    #[test]
    fn black_is_index_zero() -> T {
        let mut a = image(2, 1, &[K, K])?;
        let p = tmp("k8").join("k.bmp");
        save_as(&mut a, &p, T8)?;
        assert!(a.dialog()?.is_none());
        let b = read(&p)?;
        assert_eq!(&b[1078..1080], &[0, 0]);
        Ok(())
    }

    #[test]
    fn white_image_no_warning() -> T {
        let mut a = fresh()?;
        let p = tmp("w4").join("w.bmp");
        save_as(&mut a, &p, T4)?;
        assert!(a.dialog()?.is_none());
        let d = decode_file(&p)?;
        assert_eq!(d.bpp, 4);
        assert!(d.index.iter().all(|&i| i == 15));
        Ok(())
    }

    #[test]
    fn warning_cancel_no_file() -> T {
        let dir = tmp("cancel4");
        let mut a = image(2, 1, &[rgb(1, 2, 3), W])?;
        save_as(&mut a, &dir.join("x.bmp"), T4)?;
        let d = expect_dialog(&a, DialogKind::MessageBox)?;
        assert!(d.text.contains("permanently change"));
        assert_eq!(d.buttons, ["OK", "Cancel"]);
        a.dialog_click("Cancel")?;
        assert!(dir_names(&dir)?.is_empty());
        assert_eq!(px(&a, 0, 0)?, rgb(1, 2, 3));
        Ok(())
    }

    #[test]
    fn memory_image_mapped() -> T {
        let mut a = image(1, 1, &[rgb(250, 10, 10)])?;
        saved_bytes(&mut a, T4)?;
        assert_eq!(px(&a, 0, 0)?, RED);
        let d = a.document()?;
        assert_eq!(d.source_depth, 4);
        assert_eq!(d.palette, Some(PAL16.to_vec()));
        Ok(())
    }
}

mod integration_embedding {
    use crate::common::*;

    #[test]
    fn copy_4x4_clipboard_bitmap() -> T {
        let pat = pattern(4, 4, 3);
        let mut a = fresh()?;
        for (i, c) in pat.iter().enumerate() {
            a.set_pixel(2 + (i % 4) as i32, 2 + (i / 4) as i32, *c)?;
        }
        select_rect(&mut a, (2, 2), (5, 5))?;
        a.menu("Edit > Copy")?;
        assert_eq!(a.clipboard()?, ClipboardContent::Image(Image { width: 4, height: 4, pixels: pat }));
        Ok(())
    }

    #[test]
    fn clipboard_roundtrip_paste() -> T {
        let pat = pattern(4, 4, 5);
        let mut a = fresh()?;
        for (i, c) in pat.iter().enumerate() {
            a.set_pixel(10 + (i % 4) as i32, 10 + (i / 4) as i32, *c)?;
        }
        select_rect(&mut a, (10, 10), (13, 13))?;
        ctrl(&mut a, 'c')?;
        ctrl(&mut a, 'v')?;
        click(&mut a, 25, 25)?;
        for (i, c) in pat.iter().enumerate() {
            assert_eq!(px(&a, (i % 4) as i32, (i / 4) as i32)?, *c);
        }
        Ok(())
    }
}
