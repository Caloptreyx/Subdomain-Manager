import { ModalProps } from '@mantine/core';
import { zod4Resolver } from 'mantine-form-zod-resolver';
import { z } from 'zod';
import Button from '@/elements/buttons/Button.tsx';
import { AdminCan } from '@/elements/Can.tsx';
import PasswordInput from '@/elements/input/PasswordInput.tsx';
import Select from '@/elements/input/Select.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import FormModal from '@/elements/modals/FormModal.tsx';
import { ModalFooter } from '@/elements/modals/Modal.tsx';
import { useModalForm } from '@/plugins/form/useModalForm.ts';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import createDomain from '../../../api/admin/createDomain.ts';
import updateDomain from '../../../api/admin/updateDomain.ts';
import type { Domain } from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';

const domainFormSchema = z.object({
  domain: z.string().min(1),
  provider: z.enum(['cloudflare', 'bunny']),
  zoneId: z.string().min(1),
  credential: z.string(),
});

type DomainFormValues = z.infer<typeof domainFormSchema>;

const emptyValues: DomainFormValues = {
  domain: '',
  provider: 'cloudflare',
  zoneId: '',
  credential: '',
};

export default function DomainCreateOrUpdateModal({
  domain,
  onSaved,
  onClose,
  ...props
}: ModalProps & {
  domain: Domain | null;
  onSaved: () => void;
  onClose: () => void;
}) {
  const { t } = useTranslations();
  const { t: tExt } = useExtTranslations();

  const editing = domain !== null;

  const { form, handleClose, handleSubmit, loading, isDirty } = useModalForm<DomainFormValues>({
    validate: zod4Resolver(domainFormSchema),
    initialValues: emptyValues,
    opened: props.opened,
    onClose,
    hydrate: () =>
      domain
        ? {
            domain: domain.domain,
            provider: domain.provider,
            zoneId: domain.zoneId,
            credential: '',
          }
        : undefined,
    onSubmit: async (values) => {
      if (editing) {
        await updateDomain(domain.uuid, {
          domain: values.domain,
          provider: values.provider,
          zoneId: values.zoneId,
          ...(values.credential ? { credential: values.credential } : {}),
        });
      } else {
        await createDomain(values);
      }
      onSaved();
    },
  });

  const provider = form.getValues().provider;

  return (
    <FormModal
      {...props}
      title={
        editing
          ? tExt('pages.admin.subdomains.domains.modal.titleEdit', {})
          : tExt('pages.admin.subdomains.domains.modal.titleCreate', {})
      }
      onSubmit={handleSubmit}
      isDirty={isDirty}
      loading={loading}
      onClose={handleClose}
    >
      <Stack gap='md'>
        <TextInput
          withAsterisk
          label={tExt('pages.admin.subdomains.domains.modal.form.domain', {})}
          {...form.getInputProps('domain')}
        />

        <Select
          withAsterisk
          label={tExt('pages.admin.subdomains.domains.modal.form.provider', {})}
          data={[
            {
              value: 'cloudflare',
              label: tExt('pages.admin.subdomains.domains.provider.cloudflare', {}),
            },
            { value: 'bunny', label: tExt('pages.admin.subdomains.domains.provider.bunny', {}) },
          ]}
          allowDeselect={false}
          {...form.getInputProps('provider')}
        />

        <TextInput
          withAsterisk
          label={tExt('pages.admin.subdomains.domains.modal.form.zoneId', {})}
          description={
            provider === 'bunny'
              ? tExt('pages.admin.subdomains.domains.modal.form.zoneIdBunny', {})
              : tExt('pages.admin.subdomains.domains.modal.form.zoneIdCloudflare', {})
          }
          {...form.getInputProps('zoneId')}
        />

        <PasswordInput
          withAsterisk={!editing}
          label={
            provider === 'bunny'
              ? tExt('pages.admin.subdomains.domains.modal.form.credentialBunny', {})
              : tExt('pages.admin.subdomains.domains.modal.form.credential', {})
          }
          placeholder={editing ? tExt('pages.admin.subdomains.domains.modal.form.credentialKeep', {}) : undefined}
          {...form.getInputProps('credential')}
        />

        <ModalFooter>
          <AdminCan action='subdomains.manage' cantSave>
            <Button
              type='submit'
              loading={loading}
              disabled={
                !form.isValid() || (editing && !isDirty) || (!editing && form.getValues().credential.trim() === '')
              }
            >
              {editing
                ? tExt('pages.admin.subdomains.domains.button.edit', {})
                : tExt('pages.admin.subdomains.domains.button.add', {})}
            </Button>
          </AdminCan>
          <Button variant='default' onClick={handleClose}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </Stack>
    </FormModal>
  );
}
