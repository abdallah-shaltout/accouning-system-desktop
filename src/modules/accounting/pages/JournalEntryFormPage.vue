<script setup lang="ts">
/**
 * Spreadsheet-grid journal entry form (docs/v2/11-journal-dashboard-insights.md A2). Cost center
 * and branch columns are shown but inert (single default branch/no cost centers until phase 9 —
 * see `DEFAULT_BRANCH_ID` in src/mocks/backend/core.ts).
 */
import { computed, nextTick, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { CircleAlert, CircleCheck, Copy, FileDown, Plus, SendHorizontal, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { useHotkeys } from '@/modules/core/controllers/useHotkeys';
import { useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, todayKey } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { round2 } from '@/modules/invoices/helpers/totals';
import { getCustomers, getSuppliers } from '@/modules/parties/services/partyService';
import type { Customer, Supplier } from '@/modules/parties/types';
import {
  accountPath,
  createJournalEntry,
  createOrUpdateJournalTemplate,
  getAccounts,
  getJournalEntry,
  getJournalTemplates,
  postJournalDraft,
  updateJournalDraft,
  type AccountWithBalance,
} from '../services/accountingService';
import type { JournalTemplate } from '../types';

interface Line {
  key: number;
  accountId?: string;
  description: string;
  debit?: number | string;
  credit?: number | string;
  partyKind?: 'customer' | 'supplier';
  partyId?: string;
  costCenterId?: string; // inert — column only until phase 9
  branchId?: string; // inert — column only until phase 9
}

const router = useRouter();
const route = useRoute();
const toast = useToast();

const draftId = computed(() => (typeof route.query.draft === 'string' ? route.query.draft : undefined));
const duplicateFrom = computed(() => (typeof route.query.duplicate === 'string' ? route.query.duplicate : undefined));

const date = ref(todayKey());
const description = ref('');
const reference = ref('');
let seq = 0;
function blankLine(): Line {
  return { key: ++seq, description: '' };
}
const lines = ref<Line[]>([blankLine(), blankLine()]);
const accounts = ref<AccountWithBalance[]>([]);
const customers = ref<Customer[]>([]);
const suppliers = ref<Supplier[]>([]);
const templates = ref<JournalTemplate[]>([]);
const attachmentOwnerRef = ref(`journal:new:${Date.now()}`);
const attachmentIds = ref<string[]>([]);
const saving = ref(false);
const submitted = ref(false);
const loadingExisting = ref(false);

onMounted(async () => {
  [accounts.value, customers.value, suppliers.value, templates.value] = await Promise.all([
    getAccounts(),
    getCustomers(),
    getSuppliers(),
    getJournalTemplates(),
  ]);
  if (draftId.value) await loadExisting(draftId.value, true);
  else if (duplicateFrom.value) await loadExisting(duplicateFrom.value, false);
});

async function loadExisting(id: string, editingDraft: boolean) {
  loadingExisting.value = true;
  try {
    const entry = await getJournalEntry(id);
    date.value = entry.date.slice(0, 10);
    description.value = editingDraft ? entry.description : `نسخة من ${entry.description}`;
    lines.value = entry.lines.map((l) => ({
      key: ++seq,
      accountId: l.accountId,
      description: l.description ?? '',
      debit: l.debit || undefined,
      credit: l.credit || undefined,
      partyKind: l.partyKind,
      partyId: l.partyId,
    }));
    attachmentOwnerRef.value = editingDraft ? `journal:${id}` : `journal:new:${Date.now()}`;
  } catch (err) {
    toast.error(err);
  } finally {
    loadingExisting.value = false;
  }
}

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
  if (filled.value.some((l) => requiresParty(l) && !l.partyId))
    list.push('حساب العملاء أو الموردين يتطلب اختيار عميل أو مورد لكل سطر');
  if (difference.value !== 0) list.push('المدين لا يساوي الدائن');
  if (totalDebit.value === 0) list.push('أدخل المبالغ');
  return list;
});

// --- Spreadsheet-grid behavior (A2) -----------------------------------------------------------

/** Typing an amount on one side clears the other — a line is either debit or credit. Also handles
 * the "=" balancing shortcut: typing "=" alone in an amount cell fills the remaining difference. */
function onAmountInput(line: Line, side: 'debit' | 'credit', event: Event) {
  const input = event.target as HTMLInputElement;
  if (input.value.trim() === '=') {
    const others = lines.value.filter((l) => l !== line);
    const otherDebit = others.reduce((a, l) => a + num0(l.debit), 0);
    const otherCredit = others.reduce((a, l) => a + num0(l.credit), 0);
    const remaining = round2(side === 'debit' ? otherCredit - otherDebit : otherDebit - otherCredit);
    line[side] = remaining > 0 ? remaining : 0;
    line[side === 'debit' ? 'credit' : 'debit'] = undefined;
    nextTick(() => {
      input.value = String(line[side] ?? '');
    });
    return;
  }
  if (num0(line[side]) > 0) line[side === 'debit' ? 'credit' : 'debit'] = undefined;
}

function addLine() {
  const diff = difference.value;
  const line = blankLine();
  if (diff < 0) line.debit = -diff;
  else if (diff > 0) line.credit = diff;
  lines.value.push(line);
  return line;
}

function removeLine(line: Line) {
  if (lines.value.length <= 2) return;
  lines.value = lines.value.filter((l) => l.key !== line.key);
}

/** Ctrl+D: duplicate a line right after itself. */
function duplicateLine(line: Line) {
  const idx = lines.value.findIndex((l) => l.key === line.key);
  const copy: Line = { ...line, key: ++seq };
  lines.value.splice(idx + 1, 0, copy);
  nextTick(() => focusCell(idx + 1, 'account'));
}

// Cell grid navigation: Enter moves to the next cell (wrapping into a new row at the end).
// `account`/`party` cells are <AppCombobox> (its `open()` opens the dropdown, which is the
// nearest thing to "focus" for a listbox trigger); `description`/`debit`/`credit` are plain
// <input> elements, focused directly.
const COLUMNS = ['account', 'party', 'description', 'debit', 'credit'] as const;
type ColumnKey = (typeof COLUMNS)[number];
type CellHandle = HTMLInputElement | { open: () => void } | undefined;

function cellKey(rowIndex: number, col: ColumnKey): string {
  return `${rowIndex}:${col}`;
}
const cellEls = ref<Record<string, CellHandle>>({});
function setCellRef(rowIndex: number, col: ColumnKey, el: CellHandle) {
  cellEls.value[cellKey(rowIndex, col)] = el ?? undefined;
}

function focusCell(rowIndex: number, col: ColumnKey) {
  nextTick(() => {
    const el = cellEls.value[cellKey(rowIndex, col)];
    if (!el) return;
    if (el instanceof HTMLInputElement) el.select();
    else el.open();
  });
}

function onCellKeydown(e: KeyboardEvent, rowIndex: number, col: ColumnKey) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    const colIdx = COLUMNS.indexOf(col);
    if (colIdx < COLUMNS.length - 1) {
      focusCell(rowIndex, COLUMNS[colIdx + 1]);
    } else if (rowIndex < lines.value.length - 1) {
      focusCell(rowIndex + 1, COLUMNS[0]);
    } else {
      addLine();
      nextTick(() => focusCell(rowIndex + 1, COLUMNS[0]));
    }
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') {
    e.preventDefault();
    duplicateLine(lines.value[rowIndex]);
  }
}

/** Paste from Excel/clipboard as tab-separated rows: account code/name \t description \t debit \t credit. */
function onGridPaste(e: ClipboardEvent, rowIndex: number) {
  const text = e.clipboardData?.getData('text/plain');
  if (!text || !text.includes('\t')) return; // let a normal single-value paste through
  e.preventDefault();
  const rows = text.trim().split(/\r?\n/).map((r) => r.split('\t'));
  rows.forEach((cells, i) => {
    const targetIndex = rowIndex + i;
    while (lines.value.length <= targetIndex) lines.value.push(blankLine());
    const line = lines.value[targetIndex];
    const [accountText, desc, debitText, creditText] = cells;
    const match = accounts.value.find(
      (a) => a.code === accountText?.trim() || `${a.code} — ${a.name}` === accountText?.trim() || a.name === accountText?.trim(),
    );
    if (match) {
      line.accountId = match.id;
      onAccountChange(line);
    }
    if (desc !== undefined) line.description = desc.trim();
    if (debitText !== undefined && debitText.trim()) line.debit = round2(Number(debitText.replace(/,/g, '')) || 0);
    if (creditText !== undefined && creditText.trim()) line.credit = round2(Number(creditText.replace(/,/g, '')) || 0);
  });
  toast.success(`تم لصق ${rows.length} سطر`);
}

// --- Templates ------------------------------------------------------------------------------

function applyTemplate(templateId: string) {
  const template = templates.value.find((t) => t.id === templateId);
  if (!template) return;
  description.value = description.value || template.description || template.name;
  lines.value = template.lines.map((l) => ({
    key: ++seq,
    accountId: l.accountId,
    description: l.description ?? '',
    debit: l.debit || undefined,
    credit: l.credit || undefined,
    partyKind: l.partyKind,
    partyId: l.partyId,
  }));
  while (lines.value.length < 2) lines.value.push(blankLine());
}

const templateOptions = computed(() => templates.value.map((t) => ({ value: t.id, label: t.name })));

// --- Save as template -------------------------------------------------------------------------

const templateModalOpen = ref(false);
const templateName = ref('');
const templateSaving = ref(false);

function openSaveAsTemplate() {
  if (filled.value.length < 2) {
    toast.error(new Error('أضف سطرين على الأقل قبل الحفظ كقالب'));
    return;
  }
  templateName.value = description.value;
  templateModalOpen.value = true;
}

async function saveAsTemplate() {
  if (!templateName.value.trim()) return;
  templateSaving.value = true;
  try {
    await createOrUpdateJournalTemplate({
      name: templateName.value.trim(),
      description: description.value,
      lines: filled.value.map((l) => ({
        accountId: l.accountId!,
        description: l.description.trim() || undefined,
        debit: num0(l.debit),
        credit: num0(l.credit),
        partyKind: l.partyKind,
        partyId: l.partyId,
      })),
    });
    toast.success('تم حفظ القالب');
    templateModalOpen.value = false;
    templates.value = await getJournalTemplates();
  } catch (err) {
    toast.error(err);
  } finally {
    templateSaving.value = false;
  }
}

// --- Save / post ------------------------------------------------------------------------------

function buildInput() {
  return {
    date: dateKeyToIso(date.value),
    description: description.value,
    reference: reference.value || undefined,
    lines: filled.value.map((l) => ({
      accountId: l.accountId!,
      description: l.description.trim() || undefined,
      debit: num0(l.debit),
      credit: num0(l.credit),
      partyKind: l.partyKind,
      partyId: l.partyId,
    })),
    attachmentIds: [...attachmentIds.value],
  };
}

async function submit(mode: 'draft' | 'post' | 'postAndNew') {
  submitted.value = true;
  if (mode !== 'draft' && problems.value.length) return;
  if (mode === 'draft' && !description.value.trim()) return;
  saving.value = true;
  try {
    let entry;
    if (draftId.value && mode === 'draft') {
      entry = await updateJournalDraft(draftId.value, buildInput());
      toast.success('تم حفظ المسودة');
    } else if (draftId.value && mode !== 'draft') {
      await updateJournalDraft(draftId.value, buildInput());
      entry = await postJournalDraft(draftId.value);
      toast.success('تم ترحيل القيد', entry.number);
    } else {
      entry = await createJournalEntry({ ...buildInput(), asDraft: mode === 'draft' });
      toast.success(mode === 'draft' ? 'تم حفظ المسودة' : 'تم ترحيل القيد', entry.number);
    }
    if (mode === 'postAndNew') {
      resetForm();
    } else {
      router.push(`/accounting/journal/${entry.id}`);
    }
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

function resetForm() {
  description.value = '';
  reference.value = '';
  lines.value = [blankLine(), blankLine()];
  attachmentIds.value = [];
  attachmentOwnerRef.value = `journal:new:${Date.now()}`;
  submitted.value = false;
  nextTick(() => focusCell(0, 'account'));
}

useHotkeys({
  'ctrl+Enter': () => void submit('post'),
  'ctrl+s': () => void submit('draft'),
});
</script>

<template>
  <div>
    <PageHeader
      :title="draftId ? 'تعديل مسودة قيد' : 'قيد يومية يدوي'"
      subtitle="للمصروفات، التسويات، والإيداعات — يجب أن يتساوى المدين والدائن قبل الترحيل"
      back="/accounting/journal"
    />

    <AppCard padding="sm" class="mb-4">
      <div class="grid gap-4 sm:grid-cols-[160px_1fr_220px]">
        <AppInput v-model="date" type="date" label="التاريخ" required />
        <AppInput
          v-model="description"
          label="البيان"
          required
          placeholder="مثال: سداد فاتورة الكهرباء لشهر سبتمبر"
          :error="submitted && !description.trim() ? 'أدخل بيان القيد' : undefined"
        />
        <AppInput v-model="reference" label="المرجع (اختياري)" placeholder="رقم مرجعي" />
      </div>
      <div v-if="templateOptions.length" class="mt-3 flex items-center gap-2">
        <span class="text-xs text-text-secondary">تحميل من قالب:</span>
        <AppCombobox
          :options="templateOptions"
          placeholder="اختر قالباً…"
          dense
          class="w-64"
          @update:model-value="(v) => v && applyTemplate(v)"
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
                :ref="(el: any) => setCellRef(i, 'account', el ?? undefined)"
                v-model="line.accountId"
                :options="accountOptions"
                placeholder="اختر الحساب…"
                search-placeholder="رمز أو اسم الحساب"
                dense
                :error="submitted && !line.accountId && (num0(line.debit) || num0(line.credit)) ? 'اختر الحساب' : undefined"
                @update:model-value="onAccountChange(line)"
                @keydown="onCellKeydown($event, i, 'account')"
              />
            </td>
            <td class="min-w-44 px-2 py-1.5">
              <AppCombobox
                v-if="requiresParty(line)"
                :ref="(el: any) => setCellRef(i, 'party', el ?? undefined)"
                v-model="line.partyId"
                :options="partyOptions(line.partyKind!)"
                :placeholder="line.partyKind === 'supplier' ? 'اختر المورد…' : 'اختر العميل…'"
                dense
                :error="submitted && !line.partyId ? 'مطلوب' : undefined"
                @keydown="onCellKeydown($event, i, 'party')"
              />
              <span v-else class="text-xs text-text-secondary">—</span>
            </td>
            <td class="px-2 py-1.5">
              <input
                :ref="(el: any) => setCellRef(i, 'description', el)"
                v-model="line.description"
                class="control h-8"
                @keydown="onCellKeydown($event, i, 'description')"
                @paste="onGridPaste($event, i)"
              />
            </td>
            <td class="px-2 py-1.5">
              <input
                :ref="(el: any) => setCellRef(i, 'debit', el)"
                v-model="line.debit"
                type="text"
                inputmode="decimal"
                class="control num h-8"
                @input="onAmountInput(line, 'debit', $event)"
                @keydown="onCellKeydown($event, i, 'debit')"
              />
            </td>
            <td class="px-2 py-1.5">
              <input
                :ref="(el: any) => setCellRef(i, 'credit', el)"
                v-model="line.credit"
                type="text"
                inputmode="decimal"
                class="control num h-8"
                @input="onAmountInput(line, 'credit', $event)"
                @keydown="onCellKeydown($event, i, 'credit')"
              />
            </td>
            <td class="px-2">
              <div class="flex items-center gap-0.5">
                <button
                  type="button"
                  class="rounded p-1.5 text-text-secondary hover:bg-surface-hover"
                  aria-label="تكرار السطر (Ctrl+D)"
                  title="تكرار السطر (Ctrl+D)"
                  @click="duplicateLine(line)"
                >
                  <Copy class="size-3.5" />
                </button>
                <button
                  type="button"
                  class="rounded p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger disabled:opacity-30"
                  :disabled="lines.length <= 2"
                  aria-label="حذف السطر"
                  @click="removeLine(line)"
                >
                  <Trash class="size-3.5" />
                </button>
              </div>
            </td>
          </tr>
        </tbody>
        <tfoot class="border-t border-border bg-surface">
          <tr>
            <td colspan="4" class="px-3 py-2">
              <AppButton size="sm" variant="ghost" :icon="Plus" @click="addLine">إضافة سطر</AppButton>
            </td>
            <td class="px-2 py-2 font-medium"><MoneyText :value="totalDebit" /></td>
            <td class="px-2 py-2 font-medium"><MoneyText :value="totalCredit" /></td>
            <td />
          </tr>
        </tfoot>
      </table>
    </div>

    <AppCard title="المرفقات" padding="sm" class="mt-4">
      <AttachmentField :owner-ref="attachmentOwnerRef" />
    </AppCard>

    <!-- Sticky balance bar (A2). -->
    <div class="sticky bottom-0 z-10 mt-4 rounded-xl border border-border bg-background/95 p-3 backdrop-blur">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="text-body">
          <p v-if="difference === 0 && totalDebit > 0" class="flex items-center gap-1.5 text-success">
            <CircleCheck class="size-4" /> القيد متوازن — <span class="num">Σ مدين <MoneyText :value="totalDebit" plain /></span>
            <span class="num">Σ دائن <MoneyText :value="totalCredit" plain /></span>
          </p>
          <p v-else-if="difference !== 0" class="flex items-center gap-1.5 text-danger">
            <CircleAlert class="size-4" /> الفرق: <MoneyText :value="Math.abs(difference)" /> {{ difference > 0 ? '(المدين أكبر)' : '(الدائن أكبر)' }}
          </p>
          <ul v-if="submitted && problems.length" class="mt-1 list-inside list-disc text-xs text-danger">
            <li v-for="p in problems" :key="p">{{ p }}</li>
          </ul>
        </div>
        <div class="flex flex-wrap gap-2">
          <AppButton to="/accounting/journal">إلغاء</AppButton>
          <AppButton :icon="FileDown" :loading="saving" @click="submit('draft')">حفظ كمسودة</AppButton>
          <AppButton variant="ghost" @click="openSaveAsTemplate">حفظ كقالب</AppButton>
          <AppButton
            variant="primary"
            :icon="SendHorizontal"
            :loading="saving"
            :disabled="submitted && problems.length > 0"
            kbd="Ctrl+Enter"
            @click="submit('post')"
          >
            ترحيل القيد
          </AppButton>
          <AppButton :disabled="submitted && problems.length > 0" :loading="saving" @click="submit('postAndNew')">ترحيل وجديد</AppButton>
        </div>
      </div>
    </div>

    <AppModal v-model:open="templateModalOpen" title="حفظ كقالب" size="sm">
      <AppInput v-model="templateName" label="اسم القالب" required placeholder="مثال: إيجار الفرع الشهري" />
      <template #footer>
        <AppButton @click="templateModalOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" :loading="templateSaving" @click="saveAsTemplate">حفظ</AppButton>
      </template>
    </AppModal>
  </div>
</template>
