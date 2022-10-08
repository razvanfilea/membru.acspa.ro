import {ActionIcon, Group, Space, Stack, Text} from "@mantine/core";
import React from "react";
import {ReservationRestriction} from "../../types/wrapper";
import {MdDelete} from "react-icons/md";

export default function ReservationRestrictionComponent(
    reservationRestriction: ReservationRestriction,
    userName: string | null,
    onDelete: () => Promise<void>,
) {
    return (<Group position={'apart'}>
        <Stack spacing={0}>
            <Text>Data: <b>{(new Date(reservationRestriction.date)).toLocaleDateString('ro-RO')}</b></Text>
            <Text>De la ora {reservationRestriction.start_hour}:{'00'}</Text>
            <Text size={"sm"}>Creat de {userName || reservationRestriction.user_id}</Text>

            <Space h={"xs"}/>

            <Text>Motiv: <b>{reservationRestriction.message}</b></Text>
        </Stack>

        <ActionIcon size={'lg'} color={'red'} variant={'filled'} onClick={async () => onDelete()}>
            <MdDelete size={26}/>
        </ActionIcon>
    </Group>)
}
