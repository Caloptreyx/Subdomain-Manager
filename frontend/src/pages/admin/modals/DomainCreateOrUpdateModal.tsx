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

const domainFormSchema = z
  .object({
    domain: z.string().min(1),
    provider: z.enum(['cloudflare', 'bunny', 'powerdns']),
    zoneId: z.string().min(1),
    // PowerDNS only; the URL and key are stored together as one credential,
    // so they can only be changed together.
    apiUrl: z.string(),
    credential: z.string(),
  })
  .refine((values) => values.provider !== 'powerdns' || !values.credential.trim() || values.apiUrl.trim(), {
    path: ['apiUrl'],
    message: 'Required',
  })
  .refine((values) => values.provider !== 'powerdns' || !values.apiUrl.trim() || values.credential.trim(), {
    path: ['credential'],
    message: 'Required',
  });

type DomainFormValues = z.infer<typeof domainFormSchema>;

const emptyValues: DomainFormValues = {
  domain: '',
  provider: 'cloudflare',
  zoneId: '',
  apiUrl: '',
  credential: '',
};

function packCredential(values: DomainFormValues): string {
  const credential = values.credential.trim();
  if (!credential || values.provider !== 'powerdns') {
    return credential;
  }
  return JSON.stringify({ api_url: values.apiUrl.trim(), api_key: credential });
}

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
            apiUrl: '',
            credential: '',
          }
        : undefined,
    onSubmit: async (values) => {
      const credential = packCredential(values);
      if (editing) {
        await updateDomain(domain.uuid, {
          domain: values.domain,
          provider: values.provider,
          zoneId: values.zoneId,
          ...(credential ? { credential } : {}),
        });
      } else {
        await createDomain({ domain: values.domain, provider: values.provider, zoneId: values.zoneId, credential });
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
            { value: 'powerdns', label: tExt('pages.admin.subdomains.domains.provider.powerdns', {}) },
          ]}
          allowDeselect={false}
          {...form.getInputProps('provider')}
        />

        <TextInput
          withAsterisk
          label={
            provider === 'powerdns'
              ? tExt('pages.admin.subdomains.domains.modal.form.zoneName', {})
              : tExt('pages.admin.subdomains.domains.modal.form.zoneId', {})
          }
          description={
            provider === 'powerdns'
              ? tExt('pages.admin.subdomains.domains.modal.form.zoneIdPowerdns', {})
              : provider === 'bunny'
                ? tExt('pages.admin.subdomains.domains.modal.form.zoneIdBunny', {})
                : tExt('pages.admin.subdomains.domains.modal.form.zoneIdCloudflare', {})
          }
          {...form.getInputProps('zoneId')}
        />

        {provider === 'powerdns' && (
          <TextInput
            withAsterisk={!editing}
            label={tExt('pages.admin.subdomains.domains.modal.form.apiUrl', {})}
            description={tExt('pages.admin.subdomains.domains.modal.form.apiUrlPowerdns', {})}
            placeholder={editing ? tExt('pages.admin.subdomains.domains.modal.form.apiUrlKeep', {}) : undefined}
            {...form.getInputProps('apiUrl')}
          />
        )}

        <PasswordInput
          withAsterisk={!editing}
          label={
            provider === 'cloudflare'
              ? tExt('pages.admin.subdomains.domains.modal.form.credential', {})
              : tExt('pages.admin.subdomains.domains.modal.form.credentialApiKey', {})
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
