import { ModalProps } from '@mantine/core';
import { useState } from 'react';
import { getHttpStatus, httpErrorToHuman } from '@/api/axios.ts';
import Alert from '@/elements/feedback/Alert.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import ConfirmationModal from '@/elements/modals/ConfirmationModal.tsx';
import { useToast } from '@/providers/ToastProvider.tsx';
import type { Subdomain } from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';

export default function SubdomainDeleteModal({
  subdomain,
  onDelete,
  ...props
}: Omit<ModalProps, 'children'> & {
  subdomain: Subdomain;
  onDelete: (force: boolean) => Promise<void>;
}) {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();

  const [dnsError, setDnsError] = useState<string | null>(null);

  const handleClose = () => {
    setDnsError(null);
    props.onClose();
  };

  const doConfirm = async () => {
    try {
      await onDelete(dnsError !== null);
    } catch (error) {
      if (dnsError === null && getHttpStatus(error) === 502) {
        const message = httpErrorToHuman(error);
        setDnsError(message);
        addToast(message, 'error');
        return;
      }
      throw error;
    }
  };

  return (
    <ConfirmationModal
      title={tExt('pages.server.subdomains.modal.delete.title', {})}
      confirm={
        dnsError !== null
          ? tExt('pages.server.subdomains.modal.delete.forceConfirm', {})
          : tExt('pages.server.subdomains.button.delete', {})
      }
      onConfirmed={doConfirm}
      {...props}
      onClose={handleClose}
    >
      <Stack gap='md'>
        {tExt('pages.server.subdomains.modal.delete.content', { fqdn: subdomain.fqdn }).md()}
        {dnsError !== null && (
          <Alert color='red'>{tExt('pages.server.subdomains.modal.delete.dnsFailed', { error: dnsError }).md()}</Alert>
        )}
      </Stack>
    </ConfirmationModal>
  );
}
