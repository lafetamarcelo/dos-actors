use crseo::{
    calibrations, ceo, shackhartmann::Diffractive as WFS_TYPE, shackhartmann::Geometric, Builder,
    Calibration, CrseoError, WavefrontSensor, WavefrontSensorBuilder, GMT, SH48,
};
use skyangle::Conversion;
use std::fs::File;
use std::time::Instant;

fn main() -> std::result::Result<(), CrseoError> {
    let n_sensor = 3;
    let mut gmt = ceo!(GMT);
    println!("M1 mode: {}", gmt.get_m1_mode_type());
    println!("M2 mode: {}", gmt.get_m2_mode_type());
    let wfs_blueprint = SH48::<WFS_TYPE>::new().n_sensor(n_sensor);
    let mut gs = wfs_blueprint
        .guide_stars(None)
        .on_ring(6f32.from_arcmin())
        .fwhm(6f64)
        .build()?;
    //gs.fwhm(6f64);
    println!("GS band: {}", gs.get_photometric_band());

    let mut wfs = wfs_blueprint.build().unwrap();
    let mut src = ceo!(SOURCE);
    let mut atm = ceo!(ATMOSPHERE);

    gs.through(&mut gmt).xpupil();
    println!("GS WFE RMS: {}nm", gs.wfe_rms_10e(-9)[0]);
    wfs.calibrate(&mut gs, 0.8);
    //    println!("# valid lenslet: {}", wfs.n_valid_lenslet());

    let mut gmt2wfs = Calibration::new(&gmt, &gs, SH48::<Geometric>::new().n_sensor(n_sensor));
    let mirror = vec![calibrations::Mirror::M2];
    let segments = vec![vec![calibrations::Segment::Rxyz(1e-6, Some(0..2))]; 7];
    let now = Instant::now();
    gmt2wfs.calibrate(
        mirror,
        segments,
        calibrations::ValidLensletCriteria::OtherSensor(&mut wfs),
        //calibrations::ValidLensletCriteria::Threshold(Some(0.8)),
    );
    println!(
        "GTM 2 WFS calibration [{}x{}] in {}s",
        gmt2wfs.n_data,
        gmt2wfs.n_mode,
        now.elapsed().as_secs()
    );
    let poke_sum = gmt2wfs.poke.from_dev().iter().sum::<f32>();
    println!("Poke sum: {}", poke_sum);

    gmt.m2_segment_state(2, &[0., 0.0, 0.], &[1e-6, 0.0, 0.]);
    gmt.m2_segment_state(5, &[0., 0.0, 0.], &[0., 1e-6, 0.]);
    gmt.m2_segment_state(7, &[0., 0.0, 0.], &[1e-6, 1e-6, 0.]);
    wfs.reset();
    gs.through(&mut gmt).xpupil().through(&mut wfs);
    wfs.process();

    println!(
        "WFS data: {}",
        Vec::<f32>::from(wfs.get_data()).iter().sum::<f32>()
    );

    let a = gmt2wfs.qr().solve(&mut wfs.get_data());
    let s: Vec<f32> = (&gmt2wfs.poke * &a).into();

    Vec::<f32>::from(a)
        .into_iter()
        .map(|x| x * 1e6)
        .collect::<Vec<f32>>()
        .chunks(2)
        .enumerate()
        .for_each(|x| println!("#{}: [{:+0.1},{:+0.1}]", 1 + x.0, x.1[0], x.1[1]));
    //    println!("M2 TT: {:#?}", a);

    println!(
        "WFE RMS [nm] without and with atmosphere: {:.0}/{:.0}",
        src.through(&mut gmt).xpupil().wfe_rms_10e(-9)[0],
        src.through(&mut gmt)
            .xpupil()
            .through(&mut atm)
            .wfe_rms_10e(-9)[0]
    );
    Ok(())
}
