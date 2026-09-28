/**
 * Drift check (21.02-F, F-4 / plans/pending/21-rust-backend/03-domains/11-expenses.md §2): proves
 * the ts-rs-generated `expenses` DTOs (`expenses/types/gen/*`, written by `bun run bindings` from
 * `src-tauri/src/domains/expenses/dto.rs`) have exactly the same shape as the hand-written TS
 * types in `./index.ts` / `../services/expenseService.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect, Simplify } from '@/modules/core/types/contract';
import type { Expense, ExpenseCategory, ExpenseCategoryInput, ExpenseFilter, ExpenseInput, ExpensePaidFrom, RecurringExpense, RecurringExpenseInput } from './index';

import type { ExpenseRow } from '../services/expenseService';

import type { ExpenseCategory as GenExpenseCategory } from './gen/ExpenseCategory';
import type { ExpenseCategoryInput as GenExpenseCategoryInput } from './gen/ExpenseCategoryInput';
import type { ExpensePaidFrom as GenExpensePaidFrom } from './gen/ExpensePaidFrom';
import type { Expense as GenExpense } from './gen/Expense';
import type { ExpenseInput as GenExpenseInput } from './gen/ExpenseInput';
import type { ExpenseFilter as GenExpenseFilter } from './gen/ExpenseFilter';
import type { RecurringExpense as GenRecurringExpense } from './gen/RecurringExpense';
import type { RecurringExpenseInput as GenRecurringExpenseInput } from './gen/RecurringExpenseInput';
import type { ExpenseRow as GenExpenseRow } from './gen/ExpenseRow';

export type _ExpenseCategory = Expect<Equals<GenExpenseCategory, ExpenseCategory>>;
export type _ExpenseCategoryInput = Expect<Equals<GenExpenseCategoryInput, ExpenseCategoryInput>>;
export type _ExpensePaidFrom = Expect<Equals<GenExpensePaidFrom, ExpensePaidFrom>>;
export type _Expense = Expect<Equals<GenExpense, Expense>>;
export type _ExpenseInput = Expect<Equals<GenExpenseInput, ExpenseInput>>;
export type _ExpenseFilter = Expect<Equals<GenExpenseFilter, ExpenseFilter>>;
export type _RecurringExpense = Expect<Equals<GenRecurringExpense, RecurringExpense>>;
export type _RecurringExpenseInput = Expect<Equals<GenRecurringExpenseInput, RecurringExpenseInput>>;
// `ExpenseRow` = `Expense & { categoryName: string }` (intersection) — `Simplify` flattens it.
export type _ExpenseRow = Expect<Equals<GenExpenseRow, Simplify<ExpenseRow>>>;
