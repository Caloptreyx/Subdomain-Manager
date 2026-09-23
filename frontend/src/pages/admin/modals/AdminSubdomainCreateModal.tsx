import { ModalProps } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { z } from 'zod';
import getServers from '@/api/admin/servers/getServers.ts';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import { AdminCan } from '@/elements/Can.tsx';
import Select from '@/elements/input/Select.tsx';
import ServerSelect from '@/elements/input/ServerSelect.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import FormModal from '@/elements/modals/FormModal.tsx';
import { ModalFooter } from '@/elements/modals/Modal.tsx';
import Code from '@/elements/typography/Code.tsx';
import Text from '@/elements/typography/Text.tsx';
import { queryKeys } from '@/lib/queryKeys.ts';
import { adminServerSchema } from '@/lib/schemas/admin/servers.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import createSubdomain from '../../../api/admin/createSubdomain.ts';
import getDomains from '../../../api/admin/getDomains.ts';
import AllocationSelect from '../../../components/AllocationSelect.tsx';
import { SUBDOMAIN_NAME_REGEX } from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';

type AdminServer = z.infer<typeof adminServerSchema>;

export default function AdminSubdomainCreateModal({
  onCreated,
  ...props
}: Omit<ModalProps, 'children'> & { onCreated: () => void }) {
  const { t: tExt, tReact: tExtReact } = useExtTranslations();
  const { t } = useTranslations();
  const { addToast } = useToast();

  const [loading, setLoading] = useState(false);
  const [server, setServer] = useState<AdminServer | null>(null);
  const [domainUuid, setDomainUuid] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [allocationUuid, setAllocationUuid] = useState<string | null>(null);

  const { data: domains } = useQuery({
    queryKey: ['dev.caloptreyx.subdomains', 'admin', 'domains'],
    queryFn: () => getDomains(),
    enabled: props.opened,
  });
  const enabledDomains = (domains ?? []).filter((domain) => domain.enabled);

  useEffect(() => {
    if (!props.opened) {
      setServer(null);
      setDomainUuid(null);
      setName('');
      setAllocationUuid(null);
    }
  }, [props.opened]);

  const normalizedName = name.trim().toLowerCase();
  const nameValid = SUBDOMAIN_NAME_REGEX.test(normalizedName);
  const selectedDomain = enabledDomains.find((domain) => domain.uuid === domainUuid) ?? null;

  const doCreate = async () => {
    if (!server || !domainUuid || !allocationUuid) return;
    setLoading(true);

    try {
      await createSubdomain({ serverUuid: server.uuid, domainUuid, name: normalizedName, allocationUuid });
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
      size='lg'
      {...props}
      title={tExt('pages.server.subdomains.modal.create.title', {})}
      onSubmit={(e) => {
        e.preventDefault();
        doCreate();
      }}
    >
      <Stack gap='md'>
        <Text size='sm' c='dimmed'>
          {tExt('pages.admin.subdomains.subdomains.createNotice', {})}
        </Text>

        <ServerSelect<AdminServer>
          withAsterisk
          label={tExt('pages.admin.subdomains.subdomains.table.columns.server', {})}
          placeholder={tExt('pages.admin.subdomains.subdomains.table.columns.server', {})}
          queryKey={queryKeys.admin.servers.all()}
          fetcher={(search) => getServers(1, search)}
          value={server?.uuid ?? null}
          selectedItem={server}
          onChange={(_, selected) => {
            setServer(selected);
            setAllocationUuid(null);
          }}
        />

        <Select
          withAsterisk
          label={tExt('pages.server.subdomains.modal.create.domain', {})}
          data={enabledDomains.map((domain) => ({ label: domain.domain, value: domain.uuid }))}
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

        {nameValid && selectedDomain && (
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

        {server && (
          <AllocationSelect
            key={server.uuid}
            withAsterisk
            admin
            label={tExt('pages.server.subdomains.modal.create.allocation', {})}
            serverUuid={server.uuid}
            value={allocationUuid}
            onChange={setAllocationUuid}
          />
        )}

        <ModalFooter>
          <AdminCan action='subdomains.manage'>
            <Button type='submit' loading={loading} disabled={!server || !domainUuid || !nameValid || !allocationUuid}>
              {tExt('pages.server.subdomains.button.create', {})}
            </Button>
          </AdminCan>
          <Button variant='default' onClick={props.onClose}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </Stack>
    </FormModal>
  );
}
