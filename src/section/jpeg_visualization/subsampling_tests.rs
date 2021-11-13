#[cfg(test)]
mod subsampling_test {
    use std::convert::TryFrom;

    use crate::section::jpeg_visualization::page::subsampled_index_for_recovery;
    use crate::BLOCK_SIZE;
    use crate::section::jpeg_visualization::model::SubsamplingPack;

    #[test]
    pub fn test_vert_horiz_mult_calc() {
        let mut subsampling_pack: SubsamplingPack;

        test_mults_calculated_properly(SubsamplingPack { j: 4, a: 4, b: 4 }, 1, 1);
        test_mults_calculated_properly(SubsamplingPack { j: 4, a: 4, b: 0 }, 1, 2);
        test_mults_calculated_properly(SubsamplingPack { j: 4, a: 2, b: 2 }, 2, 1);
        test_mults_calculated_properly(SubsamplingPack { j: 4, a: 2, b: 0 }, 2, 2);
        test_mults_calculated_properly(SubsamplingPack { j: 4, a: 1, b: 1 }, 4, 1);


        fn test_mults_calculated_properly(subsampling_pack: SubsamplingPack, horiz_mult_expected: i8, vert_mult_expected: i8) {
            let horiz_mult: i8 = (subsampling_pack.j / subsampling_pack.a);
            let vert_mult: i8 = if subsampling_pack.b == 0 { 2_i8 } else { 1_i8 };

            assert_eq!(horiz_mult, horiz_mult_expected);
            assert_eq!(vert_mult, vert_mult_expected);
        }
    }

    #[test]
    pub fn test_recovery_index_4_4_4() {
        let horiz_mult: usize = 1;
        let vert_mult: usize = 1;

        let blk: usize = BLOCK_SIZE as usize;

        assert_eq!(subsampled_index_for_recovery(0, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(1, horiz_mult, vert_mult), 1);
        assert_eq!(subsampled_index_for_recovery(2, horiz_mult, vert_mult), 2);
        assert_eq!(subsampled_index_for_recovery(3, horiz_mult, vert_mult), 3);
        assert_eq!(subsampled_index_for_recovery(blk, horiz_mult, vert_mult), blk);
        assert_eq!(subsampled_index_for_recovery(137, horiz_mult, vert_mult), 137);
        assert_eq!(subsampled_index_for_recovery(200, horiz_mult, vert_mult), 200);
    }

    #[test]
    pub fn test_recovery_index_4_4_0() {
        let horiz_mult = 1;
        let vert_mult = 2;

        let blk: usize = BLOCK_SIZE as usize;

        assert_eq!(subsampled_index_for_recovery(0, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(1, horiz_mult, vert_mult), 1);
        assert_eq!(subsampled_index_for_recovery(blk - 1, horiz_mult, vert_mult), blk - 1);
        assert_eq!(subsampled_index_for_recovery(blk, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(blk + 1, horiz_mult, vert_mult), 1);
        assert_eq!(subsampled_index_for_recovery(2 * blk, horiz_mult, vert_mult), blk);
        assert_eq!(subsampled_index_for_recovery(2 * blk + 1, horiz_mult, vert_mult), blk + 1);
        assert_eq!(subsampled_index_for_recovery(3 * blk, horiz_mult, vert_mult), blk);
        assert_eq!(subsampled_index_for_recovery(3 * blk + 1, horiz_mult, vert_mult), blk + 1);
        assert_eq!(subsampled_index_for_recovery(4 * blk, horiz_mult, vert_mult), 2 * blk);
    }

    #[test]
    pub fn test_recovery_index_4_2_2() {
        let horiz_mult: usize = 2;
        let vert_mult: usize = 1;

        let blk: usize = BLOCK_SIZE as usize;

        assert_eq!(subsampled_index_for_recovery(0, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(1, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(2, horiz_mult, vert_mult), 1);
        assert_eq!(subsampled_index_for_recovery(blk, horiz_mult, vert_mult), (blk/2));
        assert_eq!(subsampled_index_for_recovery(blk + 1, horiz_mult, vert_mult), (blk / 2));
        assert_eq!(subsampled_index_for_recovery(2 * blk, horiz_mult, vert_mult), blk);
        assert_eq!(subsampled_index_for_recovery(2 * blk + 1, horiz_mult, vert_mult), blk);
        assert_eq!(subsampled_index_for_recovery(3 * blk, horiz_mult, vert_mult), blk + blk/2);
    }

    #[test]
    pub fn test_recovery_index_4_2_0() {
        let horiz_mult = 2;
        let vert_mult = 2;

        let blk: usize = BLOCK_SIZE as usize;

        assert_eq!(subsampled_index_for_recovery(0, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(1, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(2, horiz_mult, vert_mult), 1);
        assert_eq!(subsampled_index_for_recovery(3, horiz_mult, vert_mult), 1);
        assert_eq!(subsampled_index_for_recovery(4, horiz_mult, vert_mult), 2);
        assert_eq!(subsampled_index_for_recovery(blk - 2, horiz_mult, vert_mult), blk / 2 - 1);
        assert_eq!(subsampled_index_for_recovery(blk - 1, horiz_mult, vert_mult), blk / 2 - 1);
        assert_eq!(subsampled_index_for_recovery(blk, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(blk + 1, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(2 * blk, horiz_mult, vert_mult), blk / 2);
        assert_eq!(subsampled_index_for_recovery(2 * blk + 1, horiz_mult, vert_mult), blk / 2);
        assert_eq!(subsampled_index_for_recovery(2 * blk + 2, horiz_mult, vert_mult), blk / 2 + 1);
        assert_eq!(subsampled_index_for_recovery(2 * blk + 3, horiz_mult, vert_mult), blk / 2 + 1);
        assert_eq!(subsampled_index_for_recovery(3 * blk, horiz_mult, vert_mult), blk / 2);
        assert_eq!(subsampled_index_for_recovery(3 * blk + 1, horiz_mult, vert_mult), blk / 2);
        assert_eq!(subsampled_index_for_recovery(3 * blk + 2, horiz_mult, vert_mult), blk / 2 + 1);
        assert_eq!(subsampled_index_for_recovery(3 * blk + 3, horiz_mult, vert_mult), blk / 2 + 1);
        assert_eq!(subsampled_index_for_recovery(4 * blk, horiz_mult, vert_mult), blk);
    }

    #[test]
    pub fn test_recovery_index_4_1_1() {
        let horiz_mult: usize = 4;
        let vert_mult: usize = 1;

        let blk: usize = BLOCK_SIZE as usize;

        assert_eq!(subsampled_index_for_recovery(0, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(1, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(2, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(3, horiz_mult, vert_mult), 0);
        assert_eq!(subsampled_index_for_recovery(4, horiz_mult, vert_mult), 1);
        assert_eq!(subsampled_index_for_recovery(blk, horiz_mult, vert_mult), (blk/4));
        assert_eq!(subsampled_index_for_recovery(blk + 1, horiz_mult, vert_mult), (blk / 4));
        assert_eq!(subsampled_index_for_recovery(2 * blk, horiz_mult, vert_mult), blk / 2);
        assert_eq!(subsampled_index_for_recovery(4 * blk, horiz_mult, vert_mult), blk);
    }
}
