<script setup lang="ts">
/**
 * v2 phase 13b (docs/v2/14-platform.md §6 "Approvals page (`/approvals`) for managers"): the async
 * queue for when a discount/write-off/below-cost request couldn't be approved on the spot because
 * no manager was present — see `modules/approvals/types`'s doc comment for the split with the
 * synchronous PIN dialogs, which stay the primary/common path and are untouched by this page.
 */
import { computed, ref, watch } from 'vue';
import { CircleCheck, ClipboardCheck, ShieldCheck, X } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDateTime } from '@/modules/core/helpers/format';
import { approveRequest, getApprovalRequests, rejectRequest } from '../services/approvalService';
import type { ApprovalRequest, ApprovalStatus } from '../types';

const toast = useToast();
const tab = ref<ApprovalStatus>('pending');
const { data, loading, error, reload } = useAsync(() => getApprovalRequests({ status: tab.value }));
watch(tab, reload);

const KIND_LABEL: Record<ApprovalRequest['kind'], string> = {
  discount: 'خصم يتجاوز الحد',
  write_off: 'إتلاف/تسوية مخزون',
  below_cost: 'بيع أقل من التكلفة',
};

const decisionOpen = ref(false);
const decisionTarget = ref<ApprovalRequest | null>(null);
const decisionKind = ref<'approved' | 'rejected'>('approved');
const comment = ref('');
const deciding = ref(false);
const commentError = ref('');

function openDecision(request: ApprovalRequest, kind: 'approved' | 'rejected') {
  decisionTarget.value = request;
  decisionKind.value = kind;
  comment.value = '';
  commentError.value = '';
  decisionOpen.value = true;
}

async function confirmDecision() {
  if (!decisionTarget.value) return;
  if (decisionKind.value === 'rejected' && !comment.value.trim()) {
    commentError.value = 'أدخل سبب الرفض';
    return;
  }
  deciding.value = true;
  try {
    if (decisionKind.value === 'approved') await approveRequest(decisionTarget.value.id, { comment: comment.value.trim() || undefined });
    else await rejectRequest(decisionTarget.value.id, { comment: comment.value.trim() });
    toast.success(decisionKind.value === 'approved' ? 'تم اعتماد الطلب' : 'تم رفض الطلب');
    decisionOpen.value = false;
    reload();
  } catch (err) {
    toast.error(err);
  } finally {
    deciding.value = false;
  }
}

const columns = computed<Column<ApprovalRequest>[]>(() => [
  { key: 'summary', label: 'الطلب' },
  { key: 'kind', label: 'النوع' },
  { key: 'requestedByName', label: 'مقدّم الطلب' },
  { key: 'requestedAt', label: 'التاريخ', sortable: true },
  ...(tab.value !== 'pending' ? [{ key: 'decidedByName', label: 'قرار بواسطة' } as Column<ApprovalRequest>] : []),
  ...(tab.value === 'pending' ? [{ key: 'actions', label: '' } as Column<ApprovalRequest>] : []),
]);
</script>

<template>
  <div>
    <PageHeader title="طلبات الاعتماد" subtitle="خصومات وإتلافات ومبيعات أقل من التكلفة طُلب اعتمادها دون وجود مدير في حينها" />

    <div class="mb-3">
      <SegmentedControl
        v-model="tab"
        :options="[
          { value: 'pending', label: 'قيد الانتظار' },
          { value: 'approved', label: 'معتمدة' },
          { value: 'rejected', label: 'مرفوضة' },
        ]"
      />
    </div>

    <DataTable :columns="columns" :rows="data ?? []" :loading="loading" :error="error" :empty-icon="ClipboardCheck" empty-title="لا توجد طلبات" @retry="reload">
      <template #cell-summary="{ row }">
        <RouterLink v-if="row.link" :to="row.link" class="font-medium text-primary hover:underline">{{ row.summary }}</RouterLink>
        <span v-else class="font-medium">{{ row.summary }}</span>
        <p v-if="row.requestNote" class="mt-0.5 text-tiny text-text-secondary">{{ row.requestNote }}</p>
        <p v-if="row.decisionComment" class="mt-0.5 text-tiny text-text-secondary">تعليق: {{ row.decisionComment }}</p>
      </template>
      <template #cell-kind="{ row }"><StatusBadge :label="KIND_LABEL[row.kind]" tone="neutral" /></template>
      <template #cell-requestedAt="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.requestedAt) }}</span></template>
      <template #cell-decidedByName="{ row }">
        <span v-if="row.decidedByName">{{ row.decidedByName }} <span class="num text-text-secondary">— {{ formatDateTime(row.decidedAt) }}</span></span>
        <span v-else>—</span>
      </template>
      <template #cell-actions="{ row }">
        <div class="flex justify-end gap-1.5">
          <AppButton size="sm" variant="primary" :icon="ShieldCheck" @click="openDecision(row, 'approved')">اعتماد</AppButton>
          <AppButton size="sm" variant="ghost" :icon="X" @click="openDecision(row, 'rejected')">رفض</AppButton>
        </div>
      </template>
    </DataTable>

    <AppModal v-model:open="decisionOpen" :title="decisionKind === 'approved' ? 'اعتماد الطلب' : 'رفض الطلب'" size="sm">
      <div v-if="decisionTarget" class="space-y-3">
        <p class="text-body">{{ decisionTarget.summary }}</p>
        <AppTextarea v-model="comment" :label="decisionKind === 'rejected' ? 'سبب الرفض' : 'تعليق (اختياري)'" :error="commentError" :rows="3" />
      </div>
      <template #footer>
        <AppButton :disabled="deciding" @click="decisionOpen = false">إلغاء</AppButton>
        <AppButton :variant="decisionKind === 'approved' ? 'primary' : 'danger'" :icon="decisionKind === 'approved' ? CircleCheck : X" :loading="deciding" @click="confirmDecision">
          {{ decisionKind === 'approved' ? 'اعتماد' : 'رفض' }}
        </AppButton>
      </template>
    </AppModal>
  </div>
</template>
