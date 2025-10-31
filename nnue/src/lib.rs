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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Network {
    input: Layer<INP, ACC>,
    accumulator_w: SVector<f32, ACC>, //these stores z values of the input layer, as in z in phi(z)
    accumulator_b: SVector<f32, ACC>, //these stores z values of the input layer, as in z in phi(z)
    hidden: Layer<{ 2 * ACC }, HID>,
    output: Layer<HID, OUT>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
// M input dimension -> N output dimension
pub struct Layer<const M: usize, const N: usize> {
    w: SMatrix<f32, N, M>,
    b: SVector<f32, N>,
    ty: PhiT,
}

//INPUT -> 2 x ACCUMULATOR -> HIDDEN -> OUTPUT
impl Network {
    const DEFAULT_IN_PHI: PhiT = PhiT::CReLU;
    const DEFAULT_OUT_PHI: PhiT = PhiT::Tanh;

    pub fn new() -> Network {
        let input: Layer<INP, ACC> = Layer::new(Network::DEFAULT_IN_PHI);
        let hidden: Layer<{ 2 * ACC }, HID> = Layer::new(Network::DEFAULT_IN_PHI);
        let accumulator_w: SVector<f32, ACC> = SVector::zeros();
        let accumulator_b: SVector<f32, ACC> = SVector::zeros();
        let output: Layer<HID, OUT> = Layer::new(Network::DEFAULT_OUT_PHI);
        return Network { input, hidden, accumulator_w, accumulator_b, output };
    }

    pub fn eval<const IS_STM_WHITE: bool>(&mut self) -> f32 {
        let (accumulator_stm, accumulator_ntm) = match IS_STM_WHITE {
            true => (self.accumulator_w, self.accumulator_b),
            false => (self.accumulator_b, self.accumulator_w),
        };

        let hidden_output: SVector<f32, HID> = ((self.hidden.w.columns_range(1..ACC) * accumulator_stm.map(self.input.ty.phi()))
            + (self.hidden.w.columns_range(ACC..2 * ACC) * accumulator_ntm.map(self.input.ty.phi()))
            + self.hidden.b)
            .map(self.hidden.ty.phi());

        return self.output.forward(hidden_output)[0];
    }

    pub fn accumulator_add<const IS_STM_WHITE: bool>(&mut self, index: usize) {
        match IS_STM_WHITE {
            true => self.accumulator_w += self.input.w.column(index),
            false => self.accumulator_b += self.input.w.column(index),
        }
    }

    pub fn accumulator_sub<const IS_STM_WHITE: bool>(&mut self, index: usize) {
        match IS_STM_WHITE {
            true => self.accumulator_w -= self.input.w.column(index),
            false => self.accumulator_b -= self.input.w.column(index),
        }
    }

    pub fn refresh_accumulator(&mut self, input: &impl InputType) {
        self.accumulator_w = self.input.linear_forward(input.to_vector_white());
        self.accumulator_b = self.input.linear_forward(input.to_vector_black());
    }

    #[cfg(not(feature = "arrayvec"))]
    pub fn refresh_accumulator_sparse(&mut self, input: &impl SparseInputType) {
        let input_white = input.to_sparse_vec_white();
        let input_black = input.to_sparse_vec_black();
        let w = self.input.w;

        self.accumulator_w = input_white.data.into_iter().fold(SVector::zeros(), |sum, i| sum + w.column(i));
        self.accumulator_b = input_black.data.into_iter().fold(SVector::zeros(), |sum, i| sum + w.column(i));
    }
    #[cfg(feature = "arrayvec")]
    pub fn refresh_accumulator_sparse(&mut self, input: &impl SparseInputType) {
        let input_white = input.to_sparse_vec_white();
        let input_black = input.to_sparse_vec_black();
        let w = self.input.w;

        self.accumulator_w = input_white.into_iter().fold(SVector::zeros(), |sum, i| sum + w.column(i));
        self.accumulator_b = input_black.into_iter().fold(SVector::zeros(), |sum, i| sum + w.column(i));
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

    fn forward(&self, input: SVector<f32, M>) -> SVector<f32, N> {
        (self.w * input + self.b).map(self.ty.phi())
    }

    fn linear_forward(&self, input: SVector<f32, M>) -> SVector<f32, N> {
        self.w * input + self.b
    }
}
