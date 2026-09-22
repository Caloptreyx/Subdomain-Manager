import { faPlus } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Center } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import { AdminCan } from '@/elements/Can.tsx';
import Table from '@/elements/data-display/Table.tsx';
import Group from '@/elements/layout/Group.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useAdminCan } from '@/plugins/usePermissions.ts';
import getDomains from '../../api/admin/getDomains.ts';
import type { Domain } from '../../lib/schemas.ts';
import { useExtTranslations } from '../../translations.ts';
import DomainRow from './DomainRow.tsx';
import DomainCreateOrUpdateModal from './modals/DomainCreateOrUpdateModal.tsx';

export const adminDomainsQueryKey = ['dev.caloptreyx.subdomains', 'admin', 'domains'] as const;

export default function DomainsTab() {
  const { t: tExt } = useExtTranslations();
  const canManage = useAdminCan('subdomains.manage');

  const [modalState, setModalState] = useState<{ open: boolean; domain: Domain | null }>({
    open: false,
    domain: null,
  });

  const {
    data: domains,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: adminDomainsQueryKey,
    queryFn: () => getDomains(),
  });

  return (
    <>
      <Group justify='flex-end' mb='md'>
        <AdminCan action='subdomains.manage'>
          <Button
            onClick={() => setModalState({ open: true, domain: null })}
            color='blue'
            leftSection={<FontAwesomeIcon icon={faPlus} />}
          >
            {tExt('pages.admin.subdomains.domains.button.add', {})}
          </Button>
        </AdminCan>
      </Group>

      <DomainCreateOrUpdateModal
        opened={modalState.open}
        onClose={() => setModalState({ open: false, domain: null })}
        domain={modalState.domain}
        onSaved={refetch}
      />

      <Table
        columns={[
          tExt('pages.admin.subdomains.domains.table.columns.domain', {}),
          tExt('pages.admin.subdomains.domains.table.columns.provider', {}),
          tExt('pages.admin.subdomains.domains.table.columns.zoneId', {}),
          tExt('pages.admin.subdomains.domains.table.columns.enabled', {}),
          tExt('pages.admin.subdomains.domains.table.columns.subdomains', {}),
          tExt('pages.admin.subdomains.domains.table.columns.created', {}),
          '',
        ]}
        loading={isLoading}
        error={error ? httpErrorToHuman(error) : null}
        pagination={{
          total: domains?.length ?? 0,
          perPage: Math.max(domains?.length ?? 0, 1),
          page: 1,
          data: domains ?? [],
        }}
        empty={
          <Center py='lg'>
            <Text c='dimmed'>{tExt('pages.admin.subdomains.domains.empty', {})}</Text>
          </Center>
        }
      >
        {domains?.map((domain) => (
          <DomainRow
            key={domain.uuid}
            domain={domain}
            canManage={canManage}
            onChanged={refetch}
            onEdit={() => setModalState({ open: true, domain })}
          />
        ))}
      </Table>
    </>
  );
}
