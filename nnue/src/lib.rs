use std::{
    io::{Error, Read, Write},
    mem::MaybeUninit,
    ptr::addr_of_mut,
    slice::{from_raw_parts, from_raw_parts_mut},
};

#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg(feature = "arrayvec")]
use arrayvec::ArrayVec;
use nalgebra::{DMatrix, DVector, SMatrix, SVector};
use rand_distr::{Normal, Uniform, num_traits::Zero};
use serde::{Deserialize, Serialize};

use crate::phi::PhiT;

mod grad;
mod phi;

pub const INPUT_DIMENSION: usize = INP;
pub use grad::Gradient;

const INP: usize = 768;
const ACC: usize = 512;
const HID: usize = 16;
const OUT: usize = 1;

pub trait InputType {
    fn to_vector_white(&self) -> DVector<f32>;
    fn to_vector_black(&self) -> DVector<f32>;
}

pub trait SparseInputType {
    fn to_sparse_vec_white(&self) -> SparseVec;
    fn to_sparse_vec_black(&self) -> SparseVec;
}

#[cfg(not(feature = "arrayvec"))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct SparseVec {
    data: Vec<usize>,
}

const MAX_CHESS_PIECE_NUMBER: usize = 32;
#[cfg(feature = "arrayvec")]
pub type SparseVec = ArrayVec<usize, MAX_CHESS_PIECE_NUMBER>;

#[cfg(not(feature = "arrayvec"))]
impl IntoIterator for SparseVec {
    type Item = usize;

    type IntoIter = <Vec<usize> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.into_iter()
    }
}

#[cfg(not(feature = "arrayvec"))]
impl SparseVec {
    pub fn new() -> Self {
        SparseVec { data: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        SparseVec { data: Vec::with_capacity(capacity) }
    }

    pub fn push(&mut self, value: usize) {
        self.push(value);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
// M input dimension -> N output dimension
pub struct Layer<const COL: usize, const ROW: usize> {
    w: DMatrix<f32>,
    b: DVector<f32>,
    ty: PhiT,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Network {
    accumulator_w: DVector<f32>,
    accumulator_b: DVector<f32>,
    input: Layer<INP, ACC>,
    hidden: Layer<{ 2 * ACC }, HID>,
    output: Layer<HID, OUT>,
}

//NETWORK_SIZE_IN_BYTES 3682832
const DATA_LEN: usize = size_of::<Network>();

impl Network {
    const DEFAULT_IN_PHI: PhiT = PhiT::CReLU;
    const DEFAULT_OUT_PHI: PhiT = PhiT::Tanh;

    pub fn new() -> Network {
        let accumulator_w: DVector<f32> = DVector::zeros(ACC);
        let accumulator_b: DVector<f32> = DVector::zeros(ACC);
        let input: Layer<INP, ACC> = Layer::new(Network::DEFAULT_IN_PHI);
        let hidden: Layer<{ 2 * ACC }, HID> = Layer::new(Network::DEFAULT_IN_PHI);
        let output: Layer<HID, OUT> = Layer::new(Network::DEFAULT_OUT_PHI);
        Network { accumulator_w, accumulator_b, input, hidden, output }
    }

    pub fn update_grad(&mut self, grad: Gradient, r: f32) {
        self.input.w += r * grad.input_dw;
        self.input.b += r * grad.input_db;
        self.hidden.w += r * grad.hidden_dw;
        self.hidden.b += r * grad.hidden_db;
        self.output.w += r * grad.output_dw;
        self.output.b += r * grad.output_db;
    }

    //pub fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
    //    //from viridithas
    //    let ptr: *const Network = &*self;
    //    writer.write_all(unsafe { from_raw_parts(ptr.cast::<u8>(), DATA_LEN) })?;
    //    Ok(())
    //}
    //
    //pub fn read(reader: &mut impl Read) -> Result<Network, Error> {
    //    //from viridithas
    //    let data = {
    //        let mut net: Box<MaybeUninit<NetworkData>> = Box::new(MaybeUninit::uninit());
    //        let mem: &mut [u8] = unsafe { from_raw_parts_mut(net.as_mut_ptr().cast::<u8>(), DATA_LEN) };
    //        reader.read_exact(mem)?;
    //        unsafe { net.assume_init() }
    //    };
    //
    //    Ok(Network { data })
    //}

    pub fn eval<const IS_STM_WHITE: bool>(&mut self) -> f32 {
        let (accumulator_stm, accumulator_ntm) = match IS_STM_WHITE {
            true => (&self.accumulator_w, &self.accumulator_b),
            false => (&self.accumulator_b, &self.accumulator_w),
        };

        let hidden_output = ((self.hidden.w.columns_range(0..ACC) * accumulator_stm.map(self.input.ty.phi()))
            + (self.hidden.w.columns_range(ACC..2 * ACC) * accumulator_ntm.map(self.input.ty.phi()))
            + &self.hidden.b)
            .map(self.hidden.ty.phi());

        return self.output.phi(&hidden_output)[0];
    }

    pub fn accumulator_add<const IS_WHITE: bool>(&mut self, index: usize) {
        match IS_WHITE {
            true => self.accumulator_w += self.input.w.column(index),
            false => self.accumulator_b += self.input.w.column(index),
        }
    }

    pub fn accumulator_sub<const IS_WHITE: bool>(&mut self, index: usize) {
        match IS_WHITE {
            true => self.accumulator_w -= self.input.w.column(index),
            false => self.accumulator_b -= self.input.w.column(index),
        }
    }

    //corresponds to moving a piece
    pub fn accumulator_addsub<const IS_WHITE: bool>(&mut self, add_index: usize, sub_index: usize) {
        let accumulator = match IS_WHITE {
            true => &mut self.accumulator_w,
            false => &mut self.accumulator_b,
        };
        let mut i = 0;
        while i < ACC {
            //m[(r,c)]
            accumulator[i] = self.input.w[(i, add_index)] - self.input.w[(i, sub_index)];
            i += 1;
        }
        //*accumulator += self.input.w.column(add_index) - self.input.w.column(sub_index);
    }

    //corresponds to capturing a piece
    pub fn accumulator_addsubsub<const IS_WHITE: bool>(&mut self, add_index: usize, sub_index1: usize, sub_index2: usize) {
        let accumulator = match IS_WHITE {
            true => &mut self.accumulator_w,
            false => &mut self.accumulator_b,
        };
        let mut i = 0;
        while i < ACC {
            //m[(r,c)]
            accumulator[i] = self.input.w[(i, add_index)] - self.input.w[(i, sub_index1)] - self.input.w[(i, sub_index2)];
            i += 1;
        }
        //*accumulator += self.input.w.column(add_index) - self.input.w.column(sub_index1) - self.input.w.column(sub_index2);
    }

    //corresponds to capturing a piece
    pub fn accumulator_addaddsub<const IS_WHITE: bool>(&mut self, add_index1: usize, add_index2: usize, sub_index: usize) {
        let accumulator = match IS_WHITE {
            true => &mut self.accumulator_w,
            false => &mut self.accumulator_b,
        };
        let mut i = 0;
        while i < ACC {
            //m[(r,c)]
            accumulator[i] = self.input.w[(i, add_index1)] + self.input.w[(i, add_index2)] - self.input.w[(i, sub_index)];
            i += 1;
        }
        //*accumulator += self.input.w.column(add_index1) + self.input.w.column(add_index2) - self.input.w.column(sub_index);
    }

    pub fn refresh_accumulator(&mut self, input: &impl InputType) {
        self.accumulator_w = self.input.linear_forward(&input.to_vector_white());
        self.accumulator_b = self.input.linear_forward(&input.to_vector_black());
    }

    #[cfg(not(feature = "arrayvec"))]
    pub fn refresh_accumulator_sparse(&mut self, input: &impl SparseInputType) {
        let input_white = input.to_sparse_vec_white();
        let input_black = input.to_sparse_vec_black();
        let w = self.input.w;

        self.accumulator_w = input_white.into_iter().fold(self.accumulator_w.clone(), |sum, i| sum + w.column(i));
        self.accumulator_b = input_black.into_iter().fold(self.accumulator_b.clone(), |sum, i| sum + w.column(i));
    }

    #[cfg(feature = "arrayvec")]
    pub fn refresh_accumulator_sparse(&mut self, input: &impl SparseInputType) {
        let input_white = input.to_sparse_vec_white();
        let input_black = input.to_sparse_vec_black();
        let w = &self.input.w;

        self.accumulator_w = input_white.into_iter().fold(self.input.b.clone(), |sum, i| sum + w.column(i));
        self.accumulator_b = input_black.into_iter().fold(self.input.b.clone(), |sum, i| sum + w.column(i));
    }

    #[inline(always)]
    pub fn backward_prop_sparse(&mut self, in_stm: SparseVec, in_ntm: SparseVec, target: DVector<f32>, r: f32) -> Gradient {
        let mut stm: DVector<f32> = DVector::zeros(INP);
        let mut ntm: DVector<f32> = DVector::zeros(INP);

        for index in in_stm {
            stm[index] = 1.0;
        }

        for index in in_ntm {
            ntm[index] = 1.0;
        }

        self.backward_prop(stm, ntm, target, r)
    }

    pub fn backward_prop(&mut self, stm: DVector<f32>, ntm: DVector<f32>, target: DVector<f32>, r: f32) -> Gradient {
        let accumulator_stm = self.input.linear_forward(&stm);
        let accumulator_ntm = self.input.linear_forward(&ntm);
        let mut accumulator: DVector<f32> = DVector::zeros(2 * ACC);
        for i in 0..ACC {
            accumulator[i] = accumulator_stm[i];
            accumulator[ACC + i] = accumulator_ntm[i];
        }
        //let input_output_stm = self.input.linear_forward(&stm).map(self.input.ty.phi());
        //let input_output_ntm = self.input.linear_forward(&ntm).map(self.input.ty.phi());
        let input_output = accumulator.map(self.input.ty.phi());
        let hidden_linear = &self.hidden.w * &input_output + &self.hidden.b;
        let hidden_output = hidden_linear.map(self.hidden.ty.phi());
        //let hidden_output = (self.hidden.w.columns_range(0..ACC) * input_output_stm)
        //    + (self.hidden.w.columns_range(ACC..2 * ACC) * input_output_ntm)
        //    + &self.hidden.b.map(self.hidden.ty.phi());
        let output_linear = self.output.linear_forward(&hidden_output);
        let output_dphida = r.abs() * (output_linear.map(self.output.ty.phi()) - target);

        let mut grad = Gradient::zeros();

        // hidden_layer -> output_layer
        let output_dphidz = output_dphida.component_mul(&output_linear.map(self.output.ty.dphi()));
        let output_dzdw = hidden_output;
        let hidden_dphida = self.output.w.tr_mul(&output_dphidz);
        grad.output_dw = &output_dphidz * output_dzdw.transpose();
        grad.output_db = output_dphidz;

        // input_layer -> hidden_layer
        let hidden_dphidz = hidden_dphida.component_mul(&hidden_linear.map(self.hidden.ty.dphi()));
        let hidden_dzdw = input_output;
        let input_dphida = self.hidden.w.tr_mul(&hidden_dphidz);
        grad.hidden_dw = &hidden_dphidz * hidden_dzdw.transpose();
        grad.hidden_db = hidden_dphidz;

        // input -> input_layer
        let input_dphidz_stm: _ = input_dphida.rows(0, ACC).component_mul(&accumulator_stm.map(self.input.ty.dphi()));
        let input_dphidz_ntm: _ = input_dphida.rows(ACC, ACC).component_mul(&accumulator_ntm.map(self.input.ty.dphi()));
        let input_dzdw_stm = stm;
        let input_dzdw_ntm = ntm;

        grad.input_dw = (&input_dphidz_stm * input_dzdw_stm.transpose() + &input_dphidz_ntm * input_dzdw_ntm.transpose()) / 2.0;
        grad.input_db = (input_dphidz_stm + input_dphidz_ntm) / 2.0;

        return grad;
    }

    pub fn regularization_term(&self, lambda: f32) -> Gradient {
        let mut grad = Gradient::zeros();
        grad.input_dw = lambda.abs() * &self.input.w;
        grad.input_db = lambda.abs() * &self.input.b;
        grad.hidden_dw = lambda.abs() * &self.hidden.w;
        grad.hidden_db = lambda.abs() * &self.hidden.b;
        grad.output_dw = lambda.abs() * &self.output.w;
        grad.output_db = lambda.abs() * &self.output.b;
        return grad;
    }
}

impl<const COL: usize, const ROW: usize> Layer<COL, ROW> {
    fn new(ty: PhiT) -> Layer<COL, ROW> {
        let he: Normal<f32> = Normal::new(0.0, f32::sqrt(2.0 / ROW as f32)).unwrap();
        let glorot: Uniform<f32> = Uniform::new(-f32::sqrt(6.0 / ((COL + ROW) as f32)), f32::sqrt(6.0 / ((COL + ROW) as f32))).unwrap();
        let w: DMatrix<f32> = match &ty {
            PhiT::Tanh => DMatrix::from_distribution(ROW, COL, &glorot, &mut rand::rng()),
            _ => DMatrix::from_distribution(ROW, COL, &he, &mut rand::rng()),
        };

        let b = DVector::zeros(ROW);
        Layer { w, b, ty }
    }

    fn dphi(&self, input: &DVector<f32>) -> DVector<f32> {
        (&self.w * input + &self.b).map(self.ty.dphi())
    }

    fn phi(&self, input: &DVector<f32>) -> DVector<f32> {
        (&self.w * input + &self.b).map(self.ty.phi())
    }

    fn linear_forward(&self, input: &DVector<f32>) -> DVector<f32> {
        &self.w * input + &self.b
    }
}
