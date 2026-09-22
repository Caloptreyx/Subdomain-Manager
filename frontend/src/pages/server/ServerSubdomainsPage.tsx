import { faPlus } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Center } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import { ServerCan } from '@/elements/Can.tsx';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Table from '@/elements/data-display/Table.tsx';
import ConditionalTooltip from '@/elements/overlays/ConditionalTooltip.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerStore } from '@/stores/server.ts';
import getSubdomains from '../../api/server/getSubdomains.ts';
import { useExtTranslations } from '../../translations.ts';
import SubdomainCreateModal from './modals/SubdomainCreateModal.tsx';
import SubdomainRow from './SubdomainRow.tsx';

export const serverSubdomainsQueryKey = (serverUuid: string) =>
  ['dev.caloptreyx.subdomains', 'server', serverUuid, 'subdomains'] as const;

export default function ServerSubdomainsPage() {
  const { t: tExt } = useExtTranslations();
  const { server } = useServerStore();

  const [createOpen, setCreateOpen] = useState(false);

  const { data, isLoading, error, refetch } = useQuery({
    queryKey: serverSubdomainsQueryKey(server.uuid),
    queryFn: () => getSubdomains(server.uuid),
  });

  const count = data?.subdomains.length ?? 0;
  const limit = data?.limit ?? 0;
  const noDomains = (data?.domains.length ?? 0) === 0;
  const limitReached = count >= limit;
  const createDisabled = limitReached || noDomains;
  const disabledReason = noDomains
    ? tExt('pages.server.subdomains.tooltip.noDomains', {})
    : tExt('pages.server.subdomains.tooltip.limitReached', { limit });

  return (
    <ServerContentContainer
      title={tExt('pages.server.subdomains.title', {})}
      subtitle={tExt('pages.server.subdomains.subtitle', { count, limit })}
      contentRight={
        <ServerCan action='subdomains.create'>
          <ConditionalTooltip enabled={createDisabled} label={disabledReason}>
            <Button
              disabled={createDisabled}
              onClick={() => setCreateOpen(true)}
              color='blue'
              leftSection={<FontAwesomeIcon icon={faPlus} />}
            >
              {tExt('pages.server.subdomains.button.create', {})}
            </Button>
          </ConditionalTooltip>
        </ServerCan>
      }
    >
      <SubdomainCreateModal
        opened={createOpen}
        onClose={() => setCreateOpen(false)}
        serverUuid={server.uuid}
        domains={data?.domains ?? []}
        onCreated={refetch}
      />

      <Table
        columns={[
          tExt('pages.server.subdomains.table.columns.subdomain', {}),
          tExt('pages.server.subdomains.table.columns.allocation', {}),
          tExt('pages.server.subdomains.table.columns.records', {}),
          tExt('pages.server.subdomains.table.columns.created', {}),
          '',
        ]}
        loading={isLoading}
        error={error ? httpErrorToHuman(error) : null}
        pagination={{ total: count, perPage: Math.max(count, 1), page: 1, data: data?.subdomains ?? [] }}
        empty={
          <Center py='lg'>
            <Text c='dimmed'>{tExt('pages.server.subdomains.empty', {})}</Text>
          </Center>
        }
      >
        {data?.subdomains.map((subdomain) => (
          <SubdomainRow key={subdomain.uuid} subdomain={subdomain} serverUuid={server.uuid} onChanged={refetch} />
        ))}
      </Table>
    </ServerContentContainer>
  );
}
