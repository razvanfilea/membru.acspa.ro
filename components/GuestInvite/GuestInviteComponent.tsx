import {ActionIcon, Group, Space, Stack, Text} from "@mantine/core";
import React from "react";
import {GuestInvite} from "../../types/wrapper";
import {MdDelete} from "react-icons/md";

export default function GuestInviteComponent(
    guestInvite: GuestInvite,
    userName: string | null,
    onDelete: () => Promise<void>,
) {
    return <Group position={'apart'}>
        <Stack spacing={0}>
            <Text><b>{guestInvite.guest_name}</b></Text>

            <Space h={"xs"}/>

            <Text>Data: <b>{(new Date(guestInvite.date)).toLocaleDateString('ro-RO')}</b></Text>
            <Text>De la ora {guestInvite.start_hour}:{'00'}</Text>
            <Text size={"sm"}>Creat de {userName || guestInvite.user_id} pe {new Date(guestInvite.created_at).toLocaleDateString()}</Text>
        </Stack>

        <ActionIcon size={'lg'} color={'red'} variant={'filled'} onClick={async () => onDelete()}>
            <MdDelete size={26}/>
        </ActionIcon>
    </Group>
}
