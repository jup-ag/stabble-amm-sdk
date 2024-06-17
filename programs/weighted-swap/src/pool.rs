use anchor_lang::prelude::*;
use math::{
    fixed_math::{FixedComplement, FixedMul},
    swap_fee_math, weighted_math,
};

#[derive(AnchorSerialize, AnchorDeserialize, Eq, PartialEq, Clone, Copy)]
pub struct PoolToken {
    pub mint: Pubkey,        // immutable
    pub decimals: u8,        // immutable
    pub scaling_up: bool,    // immutable
    pub scaling_factor: u64, // immutable
    pub balance: u64,
    pub weight: u64, // immutable
}

#[account]
pub struct Pool {
    pub owner: Pubkey,
    pub vault: Pubkey,      // immutable
    pub mint: Pubkey,       // immutable
    pub authority_bump: u8, // immutable
    pub is_active: bool,
    pub invariant: u64,
    pub swap_fee: u64,
    pub tokens: Vec<PoolToken>,
    pub pending_owner: Option<Pubkey>,
}

impl Pool {
    pub fn get_normalized_weights(&self) -> Vec<u64> {
        self.tokens.iter().map(|token| token.weight).collect()
    }

    pub fn get_balances(&self) -> Vec<u64> {
        self.tokens.iter().map(|token| token.balance).collect()
    }

    pub fn get_token_index(&self, mint: Pubkey) -> usize {
        self.tokens
            .iter()
            .enumerate()
            .find(|(_, token)| token.mint == mint)
            .unwrap()
            .0
    }

    /// scaling up/down from token amount to wrapped balance amount
    pub fn calc_wrapped_amount(&self, amount: u64, token_index: usize) -> u64 {
        if self.tokens[token_index].scaling_factor == 1 {
            amount
        } else if self.tokens[token_index].scaling_up {
            amount * self.tokens[token_index].scaling_factor
        } else {
            amount / self.tokens[token_index].scaling_factor
        }
    }

    /// scaling up/down from wrapped balance amount to token amount
    pub fn calc_unwrapped_amount(&self, amount: u64, token_index: usize) -> u64 {
        if self.tokens[token_index].scaling_factor == 1 {
            amount
        } else if self.tokens[token_index].scaling_up {
            amount / self.tokens[token_index].scaling_factor
        } else {
            amount * self.tokens[token_index].scaling_factor
        }
    }

    /// round down token amount not to send the lost amount from wrapped balance amount when it scaled down
    pub fn calc_rounded_amount(&self, amount: u64, token_index: usize) -> u64 {
        if self.tokens[token_index].scaling_up {
            amount
        } else {
            amount / self.tokens[token_index].scaling_factor * self.tokens[token_index].scaling_factor
        }
    }

    /// estimated swap amount out
    pub fn get_swap_result(
        &self,
        token_in_index: usize,
        token_out_index: usize,
        amount_in: u64,
        x_amount: u64,
    ) -> (u64, u64) {
        if self.invariant == 0 {
            return (0, 0);
        }

        let swap_fee = swap_fee_math::calc_swap_fee_in_discount(self.swap_fee, x_amount);

        let wrapped_amount_in = self.calc_wrapped_amount(amount_in, token_in_index);
        let wrapped_amount_out_without_fee = weighted_math::calc_out_given_in(
            self.tokens[token_in_index].balance,
            self.tokens[token_in_index].weight,
            self.tokens[token_out_index].balance,
            self.tokens[token_out_index].weight,
            wrapped_amount_in,
        )
        .unwrap();

        let wrapped_amount_out = wrapped_amount_out_without_fee.mul_down(swap_fee.complement());
        let wrapped_amount_fee = wrapped_amount_out_without_fee - wrapped_amount_out;
        let amount_out = self.calc_unwrapped_amount(wrapped_amount_out, token_out_index);
        let amount_fee = self.calc_unwrapped_amount(wrapped_amount_fee, token_out_index);

        (amount_out, amount_fee)
    }
}
