import {Button, Group, Space, Stack, Text} from "@mantine/core";
import React from "react";
import {MdCancel} from "react-icons/md";
import {GameTable, LocationName, Reservation, ReservationStatus} from "../../types/wrapper";

function ShowStatus(reservation: Reservation, onCancel: () => Promise<void>) {
    const resStatus = reservation.status ?? ReservationStatus.Cancelled;

    switch (resStatus) {
        case ReservationStatus.Cancelled:
            return <Stack align={"center"} spacing={'xs'}>
                <MdCancel size={32}/>
                <Text weight={700}>Anulată</Text></Stack>
        case ReservationStatus.Approved:
            return <Button
                gradient={{from: 'orange', to: 'red'}} variant={"outline"}
                onClick={onCancel}>Anulează</Button>
    }
}

export default function ReservationComponent(reservation: Reservation, gameTable: GameTable, showStatus: boolean, onCancel: () => Promise<void>) {
    return (<Group position={"apart"}>
        <Stack spacing={0}>
            {gameTable.location == LocationName.Boromir &&
                <Text size={"lg"} weight={800}>{gameTable.name}</Text>
            }

            <Text>Pe data de <b>{(new Date(reservation.start_date)).toLocaleDateString('ro-RO')}</b> de la
                ora <b>{reservation.start_hour}:{'00'}</b> la <b>{reservation.start_hour + reservation.duration}:{'00'}</b></Text>

            <Space h={"xs"}/>

            <Text>Locația: <b>{gameTable.location.toUpperCase()}</b></Text>

            {gameTable.location == LocationName.Boromir &&
                <>
                    <Text>Tipul mesei: <b>{gameTable.type.toUpperCase()}</b></Text>
                    <Text>Culoarea mesei: <b>{gameTable.color.toUpperCase()}</b></Text>
                    <Text>Robot: <b>{gameTable.has_robot ? "DA" : "NU"}</b></Text>
                </>
            }
        </Stack>

        {showStatus &&
            ShowStatus(reservation, onCancel)
        }
    </Group>)
}
