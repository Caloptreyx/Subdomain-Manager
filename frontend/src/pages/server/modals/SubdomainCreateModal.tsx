import { ModalProps } from '@mantine/core';
import { useEffect, useMemo, useState } from 'react';
import { z } from 'zod';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import { ServerCan } from '@/elements/Can.tsx';
import Select from '@/elements/input/Select.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import FormModal from '@/elements/modals/FormModal.tsx';
import { ModalFooter } from '@/elements/modals/Modal.tsx';
import Code from '@/elements/typography/Code.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import createSubdomain from '../../../api/server/createSubdomain.ts';
import AllocationSelect from '../../../components/AllocationSelect.tsx';
import { domainRefSchema, SUBDOMAIN_NAME_REGEX } from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';

export default function SubdomainCreateModal({
  serverUuid,
  domains,
  allocationUuid: initialAllocationUuid,
  onCreated,
  ...props
}: ModalProps & {
  serverUuid: string;
  domains: z.infer<typeof domainRefSchema>[];
  allocationUuid?: string;
  onCreated: () => void;
}) {
  const { t: tExt, tReact: tExtReact } = useExtTranslations();
  const { t } = useTranslations();
  const { addToast } = useToast();

  const [loading, setLoading] = useState(false);
  const [domainUuid, setDomainUuid] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [allocationUuid, setAllocationUuid] = useState<string | null>(null);

  useEffect(() => {
    if (!props.opened) {
      setDomainUuid(null);
      setName('');
      setAllocationUuid(null);
    } else if (initialAllocationUuid) {
      setAllocationUuid(initialAllocationUuid);
    }
  }, [props.opened]);

  const normalizedName = name.trim().toLowerCase();
  const nameValid = SUBDOMAIN_NAME_REGEX.test(normalizedName);
  const selectedDomain = useMemo(
    () => domains.find((domain) => domain.uuid === domainUuid) ?? null,
    [domains, domainUuid],
  );

  const doCreate = async () => {
    setLoading(true);

    try {
      await createSubdomain(serverUuid, {
        domainUuid: domainUuid ?? '',
        name: normalizedName,
        allocationUuid: allocationUuid ?? '',
      });
      addToast(tExt('pages.server.subdomains.toast.created', {}), 'success');
      onCreated();
      props.onClose();
    } catch (msg) {
      addToast(httpErrorToHuman(msg), 'error');
    }
    setLoading(false);
  };

  return (
    <FormModal
      {...props}
      title={tExt('pages.server.subdomains.modal.create.title', {})}
      onSubmit={(e) => {
        e.preventDefault();
        doCreate();
      }}
    >
      <Stack gap='md'>
        <Select
          withAsterisk
          label={tExt('pages.server.subdomains.modal.create.domain', {})}
          data={domains.map((domain) => ({ label: domain.domain, value: domain.uuid }))}
          value={domainUuid}
          onChange={setDomainUuid}
          allowDeselect={false}
        />

        <TextInput
          withAsterisk
          label={tExt('pages.server.subdomains.modal.create.name', {})}
          description={tExt('pages.server.subdomains.modal.create.nameDescription', {})}
          value={name}
          onChange={(e) => setName(e.target.value)}
          error={name.length > 0 && !nameValid}
        />

        {normalizedName.length > 0 && nameValid && selectedDomain && (
          <Text size='sm' c='dimmed'>
            {tExtReact('pages.server.subdomains.modal.create.preview', {
              fqdn: (
                <Code>
                  {normalizedName}.{selectedDomain.domain}
                </Code>
              ),
            })}
          </Text>
        )}

        <AllocationSelect
          withAsterisk
          label={tExt('pages.server.subdomains.modal.create.allocation', {})}
          serverUuid={serverUuid}
          value={allocationUuid}
          onChange={setAllocationUuid}
        />

        <ModalFooter>
          <ServerCan action='subdomains.create'>
            <Button type='submit' loading={loading} disabled={!domainUuid || !nameValid || !allocationUuid}>
              {tExt('pages.server.subdomains.button.create', {})}
            </Button>
          </ServerCan>
          <Button variant='default' onClick={props.onClose}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </Stack>
    </FormModal>
  );
}
