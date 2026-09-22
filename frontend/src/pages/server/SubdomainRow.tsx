import { faLink, faTrash } from '@fortawesome/free-solid-svg-icons';
import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';

import CopyOnClick from '@/elements/CopyOnClick.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import ContextMenu, { ContextMenuToggle } from '@/elements/overlays/ContextMenu.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Code from '@/elements/typography/Code.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import deleteSubdomain from '../../api/server/deleteSubdomain.ts';
import type { Subdomain } from '../../lib/schemas.ts';
import { useExtTranslations } from '../../translations.ts';
import SubdomainAllocationModal from './modals/SubdomainAllocationModal.tsx';
import SubdomainDeleteModal from './modals/SubdomainDeleteModal.tsx';
import { serverSubdomainsQueryKey } from './ServerSubdomainsPage.tsx';

export default function SubdomainRow({
  subdomain,
  serverUuid,
  onChanged,
}: {
  subdomain: Subdomain;
  serverUuid: string;
  onChanged: () => void;
}) {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const queryClient = useQueryClient();

  const [openModal, setOpenModal] = useState<'allocation' | 'delete' | null>(null);
  const canUpdate = useServerCan('subdomains.update');
  const canDelete = useServerCan('subdomains.delete');

  const refresh = () => {
    onChanged();
    queryClient.invalidateQueries({ queryKey: serverSubdomainsQueryKey(serverUuid) });
  };

  const doDelete = async (force: boolean): Promise<void> => {
    await deleteSubdomain(serverUuid, subdomain.uuid, force);
    addToast(tExt('pages.server.subdomains.toast.deleted', {}), 'success');
    setOpenModal(null);
    refresh();
  };

  return (
    <>
      <SubdomainAllocationModal
        opened={openModal === 'allocation'}
        onClose={() => setOpenModal(null)}
        serverUuid={serverUuid}
        subdomain={subdomain}
        onChanged={refresh}
      />
      <SubdomainDeleteModal
        opened={openModal === 'delete'}
        onClose={() => setOpenModal(null)}
        subdomain={subdomain}
        onDelete={doDelete}
      />

      <ContextMenu
        items={[
          {
            type: 'action',
            icon: faLink,
            label: tExt('pages.server.subdomains.button.changeAllocation', {}),
            onClick: () => setOpenModal('allocation'),
            color: 'gray',
            canAccess: canUpdate,
          },
          {
            type: 'action',
            icon: faTrash,
            label: tExt('pages.server.subdomains.button.delete', {}),
            onClick: () => setOpenModal('delete'),
            color: 'red',
            canAccess: canDelete,
          },
        ]}
      >
        {({ items, openMenu }) => (
          <TableRow
            onContextMenu={(e) => {
              e.preventDefault();
              openMenu(e.clientX, e.clientY);
            }}
          >
            <TableData>
              <CopyOnClick content={subdomain.fqdn}>
                <Code>{subdomain.fqdn}</Code>
              </CopyOnClick>
            </TableData>

            <TableData>
              {subdomain.allocation ? (
                <Code>
                  {subdomain.allocation.ipAlias ?? subdomain.allocation.ip}:{subdomain.allocation.port}
                </Code>
              ) : (
                <Tooltip label={tExt('pages.server.subdomains.tooltip.unknownAllocation', {})}>
                  <Badge color='yellow'>{tExt('pages.server.subdomains.badge.unknown', {})}</Badge>
                </Tooltip>
              )}
            </TableData>

            <TableData>
              <Tooltip
                label={
                  <Stack gap={2}>
                    {subdomain.records.map((record, index) => (
                      <Code key={index}>
                        {record.recordType} {record.name} -&gt; {record.content}
                      </Code>
                    ))}
                    {subdomain.records.length === 0 && <Code>-</Code>}
                  </Stack>
                }
              >
                <Badge color='gray'>{subdomain.records.length}</Badge>
              </Tooltip>
            </TableData>

            <TableData>
              <FormattedTimestamp timestamp={subdomain.created} />
            </TableData>

            <ContextMenuToggle items={items} openMenu={openMenu} />
          </TableRow>
        )}
      </ContextMenu>
    </>
  );
}
