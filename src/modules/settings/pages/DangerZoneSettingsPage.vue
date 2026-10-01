<script setup lang="ts">
/**
 * ACC-0035: a dedicated, discoverable home for destructive dev-only actions — previously only
 * reachable from the sidebar user menu's dev section. Dev desktop builds only (the tab itself is
 * hidden otherwise, `SettingsTabs.vue`); both actions below are also gated server-side (refuse in a
 * release build) and require re-entering a manager/admin password, so this page is not the only
 * guard — it's the discoverable front door to guards that already exist.
 *
 * The wipe action is two steps on purpose (a plain confirm dialog is too easy to click through by
 * habit for something this irreversible): (1) type the exact phrase "حذف كل شيء" to prove intent,
 * (2) re-enter a manager/admin password via the same `verifyManagerPin` the inventory
 * approval-threshold dialog uses (`ApprovalPinDialog.vue`) — it only checks the password, it never
 * switches the active session.
 */
import { ref, watch } from 'vue';
import { AlertTriangle, History, Trash2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import SettingsPage from '@/modules/core/components/layouts/SettingsPage.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { verifyManagerPin } from '@/modules/users/services/authService';
import { clearLegacySnapshot, wipeBusinessData } from '@/modules/setup/services/legacyImportService';
import SettingsTabs from '../components/SettingsTabs.vue';

const CONFIRM_PHRASE = 'حذف كل شيء';

const auth = useAuthStore();
const confirm = useConfirm();
const toast = useToast();

const wipeModalOpen = ref(false);
const phrase = ref('');
const username = ref('');
const password = ref('');
const error = ref('');
const busy = ref(false);
const legacyBusy = ref(false);

watch(wipeModalOpen, (open) => {
  if (open) {
    phrase.value = '';
    username.value = auth.user?.username ?? '';
    password.value = '';
    error.value = '';
  }
});

async function submitWipe() {
  error.value = '';
  if (phrase.value.trim() !== CONFIRM_PHRASE) {
    error.value = `اكتب العبارة بالضبط: ${CONFIRM_PHRASE}`;
    return;
  }
  if (!username.value.trim() || !password.value) {
    error.value = 'أدخل اسم المستخدم وكلمة المرور';
    return;
  }
  busy.value = true;
  try {
    await verifyManagerPin(username.value, password.value);
    await wipeBusinessData();
    wipeModalOpen.value = false;
    toast.info('تم حذف البيانات');
    window.location.reload();
  } catch (err) {
    error.value = errorMessage(err);
  } finally {
    busy.value = false;
  }
}

async function onClearLegacySnapshot() {
  const ok = await confirm({
    title: 'حذف بيانات الإصدار السابق المخزنة على هذا الجهاز',
    message: 'سيتم حذف بيانات الإصدار السابق التي يعرضها زر "استيراد بياناتك من الإصدار السابق" في شاشة الترحيب نهائياً من هذا الجهاز. هذا لا يؤثر على قاعدة البيانات الحالية.',
    confirmText: 'حذف هذه البيانات',
    danger: true,
  });
  if (!ok) return;
  legacyBusy.value = true;
  try {
    await clearLegacySnapshot();
    toast.info('تم حذف بيانات الإصدار السابق');
  } catch (err) {
    toast.error(err);
  } finally {
    legacyBusy.value = false;
  }
}
</script>

<template>
  <SettingsPage title="الإعدادات" subtitle="إجراءات خطرة — وضع التطوير فقط">
    <template #nav><SettingsTabs /></template>

    <div class="space-y-5">
      <AppCard title="حذف كل البيانات والبدء من جديد" class="border-danger/30">
        <div class="flex items-start gap-3">
          <AlertTriangle class="mt-0.5 size-5 shrink-0 text-danger" />
          <div class="space-y-3">
            <p class="text-body text-text-secondary">
              يحذف هذا الإجراء كل بيانات الشركة الحالية نهائياً — الفواتير، العملاء، الموردون،
              المنتجات، القيود المحاسبية، كل شيء — ويعيدك لشاشة البداية. استخدمه فقط إذا كنت جربت
              البيانات التجريبية وتريد البدء بشركة حقيقية فارغة، أو تريد تجربة بيانات تجريبية جديدة من
              الصفر.
            </p>
            <p class="text-caption text-text-secondary">متاح فقط في نسخة التطوير — لا يظهر هذا الخيار في النسخة النهائية التي يستخدمها العميل.</p>
            <AppButton variant="danger-solid" @click="wipeModalOpen = true">حذف كل شيء والبدء من جديد</AppButton>
          </div>
        </div>
      </AppCard>

      <AppCard title="بيانات الإصدار السابق المخزنة على هذا الجهاز" class="border-border">
        <div class="flex items-start gap-3">
          <History class="mt-0.5 size-5 shrink-0 text-text-secondary" />
          <div class="space-y-3">
            <p class="text-body text-text-secondary">
              إذا كان هذا الجهاز قد استُخدم قبل ربطه بقاعدة البيانات الحالية، قد تبقى عليه نسخة قديمة
              من البيانات تظهر كعرض "استيراد بياناتك من الإصدار السابق" في شاشة الترحيب. هذا الإجراء
              يحذف تلك النسخة القديمة نهائياً من هذا الجهاز فقط — لا علاقة له بقاعدة البيانات الحالية.
            </p>
            <AppButton variant="danger" :loading="legacyBusy" @click="onClearLegacySnapshot">
              <Trash2 class="size-4" /> حذف بيانات الإصدار السابق من هذا الجهاز
            </AppButton>
          </div>
        </div>
      </AppCard>
    </div>

    <AppModal v-model:open="wipeModalOpen" title="تأكيد حذف كل البيانات" size="sm" :persistent="busy">
      <div class="mb-4 flex items-start gap-2.5 rounded-lg bg-danger/10 p-3 text-body text-danger">
        <AlertTriangle class="mt-0.5 size-4 shrink-0" />
        <span>هذا الإجراء نهائي ولا يمكن التراجع عنه. تحقق أنك في الجهاز الصحيح قبل المتابعة.</span>
      </div>
      <form class="space-y-4" novalidate @submit.prevent="submitWipe">
        <AppInput v-model="phrase" :label="`اكتب «${CONFIRM_PHRASE}» للتأكيد`" ltr autofocus />
        <AppInput v-model="username" label="اسم مستخدم المدير" ltr />
        <AppInput v-model="password" label="كلمة المرور" type="password" ltr />
        <p v-if="error" class="text-xs text-danger" role="alert">{{ error }}</p>
      </form>
      <template #footer>
        <AppButton :disabled="busy" @click="wipeModalOpen = false">إلغاء</AppButton>
        <AppButton variant="danger-solid" :loading="busy" @click="submitWipe">حذف كل شيء نهائياً</AppButton>
      </template>
    </AppModal>
  </SettingsPage>
</template>
