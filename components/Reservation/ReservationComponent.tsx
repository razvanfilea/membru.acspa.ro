import {Button, Group, Space, Stack, Text} from "@mantine/core";
import React from "react";
import {MdCancel} from "react-icons/md";
import {Reservation, ReservationStatus} from "../../types/wrapper";

function ShowStatus(reservation: Reservation, onCancel: (() => Promise<void>) | null) {
    const resStatus = reservation.status ?? ReservationStatus.Cancelled;

    switch (resStatus) {
        case ReservationStatus.Cancelled:
            return <Stack align={"center"} spacing={'xs'}>
                <MdCancel size={32}/>
                <Text weight={700}>Anulată</Text></Stack>
        case ReservationStatus.Approved:
            return <>
                {onCancel != null &&
                    <Button
                        gradient={{from: 'orange', to: 'red'}} variant={"outline"}
                        onClick={onCancel}>Anulează</Button>
                }
            </>
    }
}

export default function ReservationComponent(
    reservation: Reservation,
    showStatus: boolean,
    onCancel: (() => Promise<void>) | null
) {
    return <Group position={"apart"}>
        <Stack spacing={0}>
            <Text>Pe data de <b>{(new Date(reservation.start_date)).toLocaleDateString('ro-RO')}</b> de la
                ora <b>{reservation.start_hour}:{'00'}</b> la <b>{reservation.start_hour + reservation.duration}:{'00'}</b></Text>

            <Space h={"xs"}/>

            <Text>Locația: <b>{reservation.location.toUpperCase()}</b></Text>
        </Stack>

        {showStatus &&
            ShowStatus(reservation, onCancel)
        }
    </Group>
}
