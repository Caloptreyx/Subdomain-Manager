import { ModalProps } from '@mantine/core';
import { useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import FormModal from '@/elements/modals/FormModal.tsx';
import { ModalFooter } from '@/elements/modals/Modal.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import updateSubdomain from '../../../api/server/updateSubdomain.ts';
import AllocationSelect from '../../../components/AllocationSelect.tsx';
import type { Subdomain } from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';

export default function SubdomainAllocationModal({
  serverUuid,
  subdomain,
  onChanged,
  ...props
}: ModalProps & {
  serverUuid: string;
  subdomain: Subdomain;
  onChanged: () => void;
}) {
  const { t } = useTranslations();
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();

  const [loading, setLoading] = useState(false);
  const [allocationUuid, setAllocationUuid] = useState<string | null>(null);

  useEffect(() => {
    if (!props.opened) {
      setAllocationUuid(null);
    }
  }, [props.opened]);

  const doUpdate = async () => {
    if (!allocationUuid) return;
    setLoading(true);

    try {
      await updateSubdomain(serverUuid, subdomain.uuid, { allocationUuid });
      addToast(tExt('pages.server.subdomains.toast.updated', {}), 'success');
      onChanged();
      props.onClose();
    } catch (msg) {
      addToast(httpErrorToHuman(msg), 'error');
    }
    setLoading(false);
  };

  return (
    <FormModal
      {...props}
      title={tExt('pages.server.subdomains.modal.changeAllocation.title', {})}
      onSubmit={(e) => {
        e.preventDefault();
        doUpdate();
      }}
    >
      <Stack gap='md'>
        <Text size='sm' c='dimmed'>
          {tExt('pages.server.subdomains.modal.changeAllocation.content', { fqdn: subdomain.fqdn })}
        </Text>

        <AllocationSelect
          withAsterisk
          label={tExt('pages.server.subdomains.modal.create.allocation', {})}
          serverUuid={serverUuid}
          value={allocationUuid}
          onChange={setAllocationUuid}
        />

        <ModalFooter>
          <Button type='submit' loading={loading} disabled={!allocationUuid}>
            {tExt('pages.server.subdomains.button.changeAllocation', {})}
          </Button>
          <Button variant='default' onClick={props.onClose}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </Stack>
    </FormModal>
  );
}
