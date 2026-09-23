<script setup lang="ts">
// TODO(phase 2): wire <AttachmentField> onto this form (docs/v2/14-platform.md §5 — journal entry attachments).
import { computed, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { CircleAlert, CircleCheck, Plus, Save, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { useHotkeys } from '@/modules/core/controllers/useHotkeys';
import { useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, todayKey } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { round2 } from '@/modules/invoices/helpers/totals';
import { getCustomers, getSuppliers } from '@/modules/parties/services/partyService';
import type { Customer, Supplier } from '@/modules/parties/types';
import { accountPath, createJournalEntry, getAccounts, type AccountWithBalance } from '../services/accountingService';

interface Line {
  key: number;
  accountId?: string;
  description: string;
  debit?: number;
  credit?: number;
  partyKind?: 'customer' | 'supplier';
  partyId?: string;
}

const router = useRouter();
const toast = useToast();

const date = ref(todayKey());
const description = ref('');
let seq = 0;
const lines = ref<Line[]>([
  { key: ++seq, description: '' },
  { key: ++seq, description: '' },
]);
const accounts = ref<AccountWithBalance[]>([]);
const customers = ref<Customer[]>([]);
const suppliers = ref<Supplier[]>([]);
const saving = ref(false);
const submitted = ref(false);

onMounted(async () => {
  [accounts.value, customers.value, suppliers.value] = await Promise.all([getAccounts(), getCustomers(), getSuppliers()]);
});

// Manual entries only post to leaf, allowManual accounts (control accounts like AR/AP/inventory/VAT
// are blocked or need a party — docs/v2/02-accounting-review.md B1). Pickers show the header path.
const accountOptions = computed(() =>
  accounts.value
    .filter((a) => a.active && !a.isGroup && a.allowManual)
    .map((a) => ({ value: a.id, label: `${a.code} — ${a.name}`, sublabel: accountPath(a, accounts.value), keywords: a.code })),
);

function accountOf(line: Line) {
  return accounts.value.find((a) => a.id === line.accountId);
}

function requiresParty(line: Line): boolean {
  return !!accountOf(line)?.requiresParty;
}

const partyOptions = (kind: 'customer' | 'supplier') =>
  (kind === 'customer' ? customers.value : suppliers.value).map((p) => ({ value: p.id, label: p.name }));

function onAccountChange(line: Line) {
  const account = accountOf(line);
  if (account?.requiresParty) {
    line.partyKind = account.systemRole === 'payable' ? 'supplier' : 'customer';
  } else {
    line.partyKind = undefined;
    line.partyId = undefined;
  }
}

const totalDebit = computed(() => round2(lines.value.reduce((a, l) => a + num0(l.debit), 0)));
const totalCredit = computed(() => round2(lines.value.reduce((a, l) => a + num0(l.credit), 0)));
const difference = computed(() => round2(totalDebit.value - totalCredit.value));
const filled = computed(() => lines.value.filter((l) => l.accountId && (num0(l.debit) > 0 || num0(l.credit) > 0)));

const problems = computed(() => {
  const list: string[] = [];
  if (!description.value.trim()) list.push('أدخل بيان القيد');
  if (filled.value.length < 2) list.push('القيد يحتاج سطرين على الأقل بحساب ومبلغ');
  if (lines.value.some((l) => num0(l.debit) > 0 && num0(l.credit) > 0)) list.push('السطر الواحد إما مدين أو دائن');
  if (lines.value.some((l) => !l.accountId && (num0(l.debit) > 0 || num0(l.credit) > 0))) list.push('اختر الحساب لكل سطر به مبلغ');
  if (filled.value.some((l) => requiresParty(l) && !l.partyId)) list.push('اختر العميل أو المورد للأسطر على حسابات العملاء/الموردين');
  if (difference.value !== 0) list.push('المدين لا يساوي الدائن');
  if (totalDebit.value === 0) list.push('أدخل المبالغ');
  return list;
});

function onAmount(line: Line, side: 'debit' | 'credit') {
  // Typing an amount on one side clears the other — a line is either debit or credit.
  if (num0(line[side]) > 0) line[side === 'debit' ? 'credit' : 'debit'] = undefined;
}

function addLine() {
  // Pre-fill the balancing amount on the new line.
  const diff = difference.value;
  lines.value.push({ key: ++seq, description: '', debit: diff < 0 ? -diff : undefined, credit: diff > 0 ? diff : undefined });
}

function balanceLast() {
  const last = [...lines.value].reverse().find((l) => !num0(l.debit) && !num0(l.credit)) ?? lines.value.at(-1)!;
  const diff = round2(difference.value + num0(last.debit) - num0(last.credit));
  last.debit = diff < 0 ? -diff : undefined;
  last.credit = diff > 0 ? diff : undefined;
}

async function save() {
  submitted.value = true;
  if (problems.value.length) return;
  saving.value = true;
  try {
    const entry = await createJournalEntry({
      date: dateKeyToIso(date.value),
      description: description.value,
      lines: filled.value.map((l) => ({
        accountId: l.accountId!,
        description: l.description.trim() || undefined,
        debit: l.debit ?? 0,
        credit: l.credit ?? 0,
        partyKind: l.partyKind,
        partyId: l.partyId,
      })),
    });
    toast.success('تم ترحيل القيد', entry.number);
    router.push(`/accounting/journal/${entry.id}`);
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

useHotkeys({ 'ctrl+Enter': () => void save(), 'ctrl+s': () => void save() });
</script>

<template>
  <div>
    <PageHeader title="قيد يومية يدوي" subtitle="للمصروفات، التسويات، والإيداعات — يجب أن يتساوى المدين والدائن قبل الترحيل" back="/accounting/journal" />

    <AppCard padding="sm" class="mb-4">
      <div class="grid gap-4 sm:grid-cols-[160px_1fr]">
        <AppInput v-model="date" type="date" label="التاريخ" required />
        <AppInput
          v-model="description"
          label="البيان"
          required
          placeholder="مثال: سداد فاتورة الكهرباء لشهر سبتمبر"
          :error="submitted && !description.trim() ? 'أدخل بيان القيد' : undefined"
        />
      </div>
    </AppCard>

    <div class="overflow-hidden rounded-xl border border-border">
      <table class="w-full text-body">
        <thead class="bg-surface text-xs text-text-secondary">
          <tr class="border-b border-border">
            <th class="w-10 px-3 py-2.5 text-start font-medium">#</th>
            <th class="px-2 py-2.5 text-start font-medium">الحساب</th>
            <th class="px-2 py-2.5 text-start font-medium">العميل/المورد</th>
            <th class="px-2 py-2.5 text-start font-medium">البيان (اختياري)</th>
            <th class="w-36 px-2 py-2.5 text-start font-medium">مدين</th>
            <th class="w-36 px-2 py-2.5 text-start font-medium">دائن</th>
            <th class="w-10" />
          </tr>
        </thead>
        <tbody>
          <tr v-for="(line, i) in lines" :key="line.key" class="border-b border-border last:border-0">
            <td class="px-3 py-1.5"><span class="num text-text-secondary">{{ i + 1 }}</span></td>
            <td class="min-w-60 px-2 py-1.5">
              <AppCombobox
                v-model="line.accountId"
                :options="accountOptions"
                placeholder="اختر الحساب…"
                search-placeholder="رمز أو اسم الحساب"
                dense
                :error="submitted && !line.accountId && (line.debit || line.credit) ? 'اختر الحساب' : undefined"
                @update:model-value="onAccountChange(line)"
              />
            </td>
            <td class="min-w-44 px-2 py-1.5">
              <AppCombobox
                v-if="requiresParty(line)"
                v-model="line.partyId"
                :options="partyOptions(line.partyKind!)"
                :placeholder="line.partyKind === 'supplier' ? 'اختر المورد…' : 'اختر العميل…'"
                dense
                :error="submitted && !line.partyId ? 'مطلوب' : undefined"
              />
              <span v-else class="text-xs text-text-secondary">—</span>
            </td>
            <td class="px-2 py-1.5"><input v-model="line.description" class="control h-8" /></td>
            <td class="px-2 py-1.5">
              <input v-model.number="line.debit" type="number" min="0" step="0.01" class="control h-8" @input="onAmount(line, 'debit')" />
            </td>
            <td class="px-2 py-1.5">
              <input v-model.number="line.credit" type="number" min="0" step="0.01" class="control h-8" @input="onAmount(line, 'credit')" />
            </td>
            <td class="px-2">
              <button
                type="button"
                class="rounded p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger disabled:opacity-30"
                :disabled="lines.length <= 2"
                aria-label="حذف السطر"
                @click="lines = lines.filter((l) => l.key !== line.key)"
              >
                <Trash class="size-3.5" />
              </button>
            </td>
          </tr>
        </tbody>
        <tfoot class="border-t border-border bg-surface">
          <tr>
            <td colspan="4" class="px-3 py-2">
              <div class="flex items-center gap-2">
                <AppButton size="sm" variant="ghost" :icon="Plus" @click="addLine">إضافة سطر</AppButton>
                <AppButton v-if="difference !== 0" size="sm" variant="ghost" @click="balanceLast">موازنة آخر سطر</AppButton>
              </div>
            </td>
            <td class="px-2 py-2 font-medium"><MoneyText :value="totalDebit" /></td>
            <td class="px-2 py-2 font-medium"><MoneyText :value="totalCredit" /></td>
            <td />
          </tr>
        </tfoot>
      </table>
    </div>

    <div class="mt-4 flex flex-wrap items-center justify-between gap-3">
      <div class="text-body">
        <p v-if="difference === 0 && totalDebit > 0" class="flex items-center gap-1.5 text-success"><CircleCheck class="size-4" /> القيد متوازن</p>
        <p v-else-if="difference !== 0" class="flex items-center gap-1.5 text-danger">
          <CircleAlert class="size-4" /> الفرق: <MoneyText :value="Math.abs(difference)" /> {{ difference > 0 ? '(المدين أكبر)' : '(الدائن أكبر)' }}
        </p>
        <ul v-if="submitted && problems.length" class="mt-1 list-inside list-disc text-xs text-danger">
          <li v-for="p in problems" :key="p">{{ p }}</li>
        </ul>
      </div>
      <div class="flex gap-2">
        <AppButton to="/accounting/journal">إلغاء</AppButton>
        <AppButton variant="primary" :icon="Save" :loading="saving" :disabled="submitted && problems.length > 0" kbd="Ctrl+Enter" @click="save">ترحيل القيد</AppButton>
      </div>
    </div>
  </div>
</template>
