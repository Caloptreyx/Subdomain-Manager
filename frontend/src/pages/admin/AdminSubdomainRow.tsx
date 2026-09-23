import { faLink, faTrash } from '@fortawesome/free-solid-svg-icons';
import { useState } from 'react';
import Badge from '@/elements/data-display/Badge.tsx';
import { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import TableLink from '@/elements/data-display/TableLink.tsx';
import ContextMenu, { ContextMenuToggle } from '@/elements/overlays/ContextMenu.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Code from '@/elements/typography/Code.tsx';
import { useAdminCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import deleteSubdomain from '../../api/admin/deleteSubdomain.ts';
import updateSubdomain from '../../api/admin/updateSubdomain.ts';
import type { AdminSubdomain } from '../../lib/schemas.ts';
import { useExtTranslations } from '../../translations.ts';
import SubdomainAllocationModal from '../server/modals/SubdomainAllocationModal.tsx';
import SubdomainDeleteModal from '../server/modals/SubdomainDeleteModal.tsx';

export default function AdminSubdomainRow({
  subdomain,
  onChanged,
}: {
  subdomain: AdminSubdomain;
  onChanged: () => void;
}) {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const canManage = useAdminCan('subdomains.manage');

  const [openModal, setOpenModal] = useState<'allocation' | 'delete' | null>(null);

  const doDelete = async (force: boolean): Promise<void> => {
    await deleteSubdomain(subdomain.uuid, force);
    addToast(tExt('pages.server.subdomains.toast.deleted', {}), 'success');
    setOpenModal(null);
    onChanged();
  };

  return (
    <>
      <SubdomainAllocationModal
        opened={openModal === 'allocation'}
        onClose={() => setOpenModal(null)}
        serverUuid={subdomain.server.uuid}
        subdomain={subdomain}
        admin
        update={(allocationUuid) => updateSubdomain(subdomain.uuid, allocationUuid)}
        onChanged={onChanged}
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
            canAccess: canManage,
          },
          {
            type: 'action',
            icon: faTrash,
            label: tExt('pages.server.subdomains.button.delete', {}),
            onClick: () => setOpenModal('delete'),
            color: 'red',
            canAccess: canManage,
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
              <Code>{subdomain.fqdn}</Code>
            </TableData>

            <TableData>
              <TableLink to={`/admin/servers/${subdomain.server.uuid}`}>{subdomain.server.name}</TableLink>
            </TableData>

            <TableData>
              {subdomain.allocation ? (
                <Code>
                  {subdomain.allocation.ipAlias ?? subdomain.allocation.ip}:{subdomain.allocation.port}
                </Code>
              ) : (
                <Badge color='yellow'>{tExt('pages.server.subdomains.badge.unknown', {})}</Badge>
              )}
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
