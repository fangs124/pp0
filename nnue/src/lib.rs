use std::{
    io::{Error, Read, Write},
    mem::MaybeUninit,
    ptr::addr_of_mut,
    slice::{from_raw_parts, from_raw_parts_mut},
};

#[cfg(feature = "arrayvec")]
use arrayvec::ArrayVec;
use nalgebra::{SMatrix, SVector};
use rand_distr::{Normal, Uniform, num_traits::Zero};
use serde::{Deserialize, Serialize};

use crate::phi::PhiT;

mod grad;
mod phi;

pub const INPUT_DIMENSION: usize = INP;
pub use grad::Gradient;

const INP: usize = 768;
const ACC: usize = 1024;
const HID: usize = 64;
const OUT: usize = 1;

pub trait InputType {
    fn to_vector_white(&self) -> SVector<f32, INPUT_DIMENSION>;
    fn to_vector_black(&self) -> SVector<f32, INPUT_DIMENSION>;
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
        self.data.into_iter()
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
        self.data.push(value);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct NetworkData {
    accumulator_w: SVector<f32, ACC>,
    accumulator_b: SVector<f32, ACC>,
    input: Layer<INP, ACC>,
    hidden: Layer<{ 2 * ACC }, HID>,
    output: Layer<HID, OUT>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Network {
    data: Box<NetworkData>,
}

//NETWORK_SIZE_IN_BYTES 3682832
const DATA_LEN: usize = size_of::<NetworkData>();

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
// M input dimension -> N output dimension
pub struct Layer<const M: usize, const N: usize> {
    w: SMatrix<f32, N, M>,
    b: SVector<f32, N>,
    ty: PhiT,
}

impl Network {
    pub fn new() -> Network {
        Network { data: NetworkData::new() }
    }

    pub fn update(&mut self, grad: Gradient, r: f32) {
        self.data.input.w += r * grad.data.input_dw;
        self.data.input.b += r * grad.data.input_db;
        self.data.hidden.w += r * grad.data.hidden_dw;
        self.data.hidden.b += r * grad.data.hidden_db;
        self.data.output.w += r * grad.data.output_dw;
        self.data.output.b += r * grad.data.output_db;
    }

    pub fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
        //from viridithas
        let ptr: *const NetworkData = &*self.data;
        writer.write_all(unsafe { from_raw_parts(ptr.cast::<u8>(), DATA_LEN) })?;
        Ok(())
    }

    pub fn read(reader: &mut impl Read) -> Result<Network, Error> {
        //from viridithas
        let data = {
            let mut net: Box<MaybeUninit<NetworkData>> = Box::new(MaybeUninit::uninit());
            let mem: &mut [u8] = unsafe { from_raw_parts_mut(net.as_mut_ptr().cast::<u8>(), DATA_LEN) };
            reader.read_exact(mem)?;
            unsafe { net.assume_init() }
        };

        Ok(Network { data })
    }

    pub fn eval<const IS_STM_WHITE: bool>(&mut self) -> f32 {
        let (accumulator_stm, accumulator_ntm) = match IS_STM_WHITE {
            true => (self.data.accumulator_w, self.data.accumulator_b),
            false => (self.data.accumulator_b, self.data.accumulator_w),
        };

        let hidden_output: SVector<f32, HID> = ((self.data.hidden.w.columns_range(0..ACC) * accumulator_stm.map(self.data.input.ty.phi()))
            + (self.data.hidden.w.columns_range(ACC..2 * ACC) * accumulator_ntm.map(self.data.input.ty.phi()))
            + self.data.hidden.b)
            .map(self.data.hidden.ty.phi());

        return self.data.output.phi(hidden_output)[0];
    }

    pub fn accumulator_add<const IS_WHITE: bool>(&mut self, index: usize) {
        match IS_WHITE {
            true => self.data.accumulator_w += self.data.input.w.column(index),
            false => self.data.accumulator_b += self.data.input.w.column(index),
        }
    }

    pub fn accumulator_sub<const IS_WHITE: bool>(&mut self, index: usize) {
        match IS_WHITE {
            true => self.data.accumulator_w -= self.data.input.w.column(index),
            false => self.data.accumulator_b -= self.data.input.w.column(index),
        }
    }

    //corresponds to moving a piece
    pub fn accumulator_addsub<const IS_WHITE: bool>(&mut self, add_index: usize, sub_index: usize) {
        let accumulator = match IS_WHITE {
            true => &mut self.data.accumulator_w,
            false => &mut self.data.accumulator_b,
        };
        *accumulator += self.data.input.w.column(add_index) - self.data.input.w.column(sub_index);
    }

    //corresponds to capturing a piece
    pub fn accumulator_addsubsub<const IS_WHITE: bool>(&mut self, add_index: usize, sub_index1: usize, sub_index2: usize) {
        let accumulator = match IS_WHITE {
            true => &mut self.data.accumulator_w,
            false => &mut self.data.accumulator_b,
        };
        *accumulator += self.data.input.w.column(add_index) - self.data.input.w.column(sub_index1) - self.data.input.w.column(sub_index2);
    }

    //corresponds to capturing a piece
    pub fn accumulator_addaddsub<const IS_WHITE: bool>(&mut self, add_index1: usize, add_index2: usize, sub_index: usize) {
        let accumulator = match IS_WHITE {
            true => &mut self.data.accumulator_w,
            false => &mut self.data.accumulator_b,
        };
        *accumulator += self.data.input.w.column(add_index1) + self.data.input.w.column(add_index2) - self.data.input.w.column(sub_index);
    }

    pub fn refresh_accumulator(&mut self, input: &impl InputType) {
        self.data.accumulator_w = self.data.input.linear_forward(input.to_vector_white());
        self.data.accumulator_b = self.data.input.linear_forward(input.to_vector_black());
    }

    #[cfg(not(feature = "arrayvec"))]
    pub fn refresh_accumulator_sparse(&mut self, input: &impl SparseInputType) {
        let input_white = input.to_sparse_vec_white();
        let input_black = input.to_sparse_vec_black();
        let w = self.data.input.w;

        self.data.accumulator_w = input_white.data.into_iter().fold(SVector::zeros(), |sum, i| sum + w.column(i));
        self.data.accumulator_b = input_black.data.into_iter().fold(SVector::zeros(), |sum, i| sum + w.column(i));
    }

    #[cfg(feature = "arrayvec")]
    pub fn refresh_accumulator_sparse(&mut self, input: &impl SparseInputType) {
        let input_white = input.to_sparse_vec_white();
        let input_black = input.to_sparse_vec_black();
        let w = self.data.input.w;

        self.data.accumulator_w = input_white.into_iter().fold(SVector::zeros(), |sum, i| sum + w.column(i));
        self.data.accumulator_b = input_black.into_iter().fold(SVector::zeros(), |sum, i| sum + w.column(i));
    }

    #[inline(always)]
    pub fn backward_prop_sparse(&mut self, in_stm: SparseVec, in_ntm: SparseVec, target: SVector<f32, OUT>, r: f32) -> Gradient {
        let mut stm: SVector<f32, INP> = SVector::zeros();
        let mut ntm: SVector<f32, INP> = SVector::zeros();

        for index in in_stm {
            stm[index] = 1.0;
        }

        for index in in_ntm {
            ntm[index] = 1.0;
        }

        self.backward_prop(stm, ntm, target, r)
    }

    pub fn backward_prop(&mut self, stm: SVector<f32, INP>, ntm: SVector<f32, INP>, target: SVector<f32, OUT>, r: f32) -> Gradient {
        let accumulator_stm = Box::new(self.data.input.linear_forward(stm));
        let accumulator_ntm = Box::new(self.data.input.linear_forward(ntm));
        let mut accumulator: SVector<f32, { 2 * ACC }> = SVector::zeros();
        for i in 0..ACC {
            accumulator[i] = accumulator_stm[i];
            accumulator[ACC + i] = accumulator_ntm[i];
        }

        let input_output = accumulator.map(self.data.input.ty.phi());
        let hidden_output: SVector<f32, HID> = (self.data.hidden.w * input_output + self.data.hidden.b).map(self.data.hidden.ty.phi());

        let output = self.data.output.phi(hidden_output);
        let out_dphida = r.abs() * (output - target);
        let mut grad = Gradient::zeros();

        // hidden_layer -> output_layer
        let output_dphidz = out_dphida.component_mul(&self.data.output.dphi(hidden_output));
        let output_dzdw = self.data.hidden.phi(input_output);
        let hidden_dphida = self.data.output.w.tr_mul(&output_dphidz);
        grad.data.output_dw = &output_dphidz * output_dzdw.transpose();
        grad.data.output_db = output_dphidz.clone();

        // input_layer -> hidden_layer
        let hidden_dphidz = hidden_dphida.component_mul(&self.data.hidden.dphi(input_output));
        let hidden_dzdw = input_output.clone();
        let input_dphida = Box::new(self.data.hidden.w.tr_mul(&hidden_dphidz));
        grad.data.hidden_dw = &hidden_dphidz * hidden_dzdw.transpose();
        grad.data.hidden_db = hidden_dphidz.clone();

        // input -> input_layer
        let input_dphidz_stm = Box::new(input_dphida.rows(0, ACC).component_mul(&self.data.input.dphi(stm)));
        let input_dphidz_ntm = Box::new(input_dphida.rows(ACC, 2 * ACC).component_mul(&self.data.input.dphi(ntm)));
        let input_dzdw_stm = stm;
        let input_dzdw_ntm = ntm;

        grad.data.input_dw = (*input_dphidz_stm * input_dzdw_stm.transpose() + *input_dphidz_ntm * input_dzdw_ntm.transpose()) / 2.0;
        grad.data.input_db = (*input_dphidz_stm + *input_dphidz_ntm) / 2.0;

        return grad;
    }

    pub fn regularization_term(&self, lambda: f32) -> Gradient {
        let mut grad = Gradient::zeros();
        grad.data.input_dw = lambda.abs() * self.data.input.w;
        grad.data.input_db = lambda.abs() * self.data.input.b;
        grad.data.hidden_dw = lambda.abs() * self.data.hidden.w;
        grad.data.hidden_db = lambda.abs() * self.data.hidden.b;
        grad.data.output_dw = lambda.abs() * self.data.output.w;
        grad.data.output_db = lambda.abs() * self.data.output.b;
        return grad;
    }
}

//INPUT -> 2 x ACCUMULATOR -> HIDDEN -> OUTPUT
impl NetworkData {
    const DEFAULT_IN_PHI: PhiT = PhiT::CReLU;
    const DEFAULT_OUT_PHI: PhiT = PhiT::Tanh;
    pub fn new() -> Box<NetworkData> {
        //let accumulator_w: Box<SVector<f32, ACC>> = Box::new(SVector::zeros());
        //let accumulator_b: Box<SVector<f32, ACC>> = Box::new(SVector::zeros());
        //let input: Box<Layer<INP, ACC>> = Layer::new_boxed(NetworkData::DEFAULT_IN_PHI);
        //let hidden: Box<Layer<{ 2 * ACC }, HID>> = Layer::new_boxed(NetworkData::DEFAULT_IN_PHI);
        //let output: Box<Layer<HID, OUT>> = Layer::new_boxed(NetworkData::DEFAULT_OUT_PHI);

        #[rustfmt::skip]
        let network_data = unsafe {
            let mut network: Box<MaybeUninit<NetworkData>> = Box::new(MaybeUninit::uninit());
            let ptr_accumulator_w =  { &raw mut (*network.as_mut_ptr()).accumulator_w };
            let ptr_accumulator_b =  { &raw mut (*network.as_mut_ptr()).accumulator_b };
            let ptr_input =  { &raw mut (*network.as_mut_ptr()).input };
            let ptr_hidden =  { &raw mut (*network.as_mut_ptr()).hidden };
            let ptr_output =  { &raw mut (*network.as_mut_ptr()).output };

            ptr_accumulator_w.write(SVector::<f32,ACC>::zeros());
            ptr_accumulator_b.write(SVector::<f32,ACC>::zeros());
            ptr_input.write(Layer::new(NetworkData::DEFAULT_IN_PHI));
            ptr_hidden.write(Layer::new(NetworkData::DEFAULT_IN_PHI));
            ptr_output.write(Layer::new(NetworkData::DEFAULT_OUT_PHI));
            
            network.assume_init()
        };

        network_data
    }
}

impl<const M: usize, const N: usize> Layer<M, N> {
    fn new(ty: PhiT) -> Layer<M, N> {
        let he: Normal<f32> = Normal::new(0.0, f32::sqrt(2.0 / N as f32)).unwrap();
        let glorot: Uniform<f32> = Uniform::new(-f32::sqrt(6.0 / ((M + N) as f32)), f32::sqrt(6.0 / ((M + N) as f32))).unwrap();
        let w: SMatrix<f32, N, M> = match &ty {
            PhiT::Tanh => SMatrix::from_distribution(&glorot, &mut rand::rng()),
            _ => SMatrix::from_distribution(&he, &mut rand::rng()),
        };

        let b = SVector::zeros();
        Layer { w, b, ty }
    }

    fn new_boxed(ty: PhiT) -> Box<Layer<M, N>> {
        let he: Normal<f32> = Normal::new(0.0, f32::sqrt(2.0 / N as f32)).unwrap();
        let glorot: Uniform<f32> = Uniform::new(-f32::sqrt(6.0 / ((M + N) as f32)), f32::sqrt(6.0 / ((M + N) as f32))).unwrap();
        let w: SMatrix<f32, N, M> = match &ty {
            PhiT::Tanh => SMatrix::from_distribution(&glorot, &mut rand::rng()),
            _ => SMatrix::from_distribution(&he, &mut rand::rng()),
        };

        let b = SVector::zeros();
        Box::new(Layer { w, b, ty })
    }

    fn dphi(&self, input: SVector<f32, M>) -> SVector<f32, N> {
        (self.w * input + self.b).map(self.ty.dphi())
    }
    fn phi(&self, input: SVector<f32, M>) -> SVector<f32, N> {
        (self.w * input + self.b).map(self.ty.phi())
    }

    fn linear_forward(&self, input: SVector<f32, M>) -> SVector<f32, N> {
        self.w * input + self.b
    }
}
