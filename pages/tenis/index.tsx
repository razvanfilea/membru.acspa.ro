import React from "react";
import Head from "next/head";
import {Card, Center, Divider, Grid, Group, Paper, Space, Stack, Text, Title, useMantineTheme} from "@mantine/core";
import {Reservation} from "../../model/Reservation";

function ReservationItem(reservation: Reservation): JSX.Element {
    return (<>
        <Card shadow="sm" p="sm" sx={(theme) => ({
            backgroundColor: theme.colors.green,
        })}>
            <Group spacing="xl">
                <Text size="sm">
                    {`${reservation.startHour}:00} - ${reservation.endHour}:00`}
                </Text>

                <Text size="sm">
                    {"User " + reservation.userId}
                </Text>
            </Group>

            <Space h="sm"/>

            <Center>
                <Text weight={"bold"} size="md">
                    {"Masa " + reservation.tableId}
                </Text>
            </Center>

            <Space h="xs"/>
        </Card>
    </>);
}

function addDays(date, days) {
    const result = new Date(date);
    result.setDate(result.getDate() + days);
    return result;
}

export default function ExcelsList({list}): JSX.Element {
    const theme = useMantineTheme()

    const current = new Date
    const startOfWeek = current.getDate() - current.getDay()

    const primaryTableColor = theme.colorScheme === 'dark'
        ? theme.colors.dark[8]
        : theme.colors.gray[3];
    const secondaryTableColor = theme.colorScheme === 'dark'
        ? theme.colors.dark[4]
        : theme.colors.gray[5];

    const days = ["Luni", "Marti", "Miercuri", "Joi", "Vineri"]

    function getReservationsForDay(index: number): Reservation[] {
        const dayOfIndex = addDays(startOfWeek, index);

        return list.filter(res => {
            return (new Date(res.date)).getDay() == dayOfIndex.getDay();
        });
    }

    const rows = days.map((day, index) => (
        <Grid.Col span={2} key={day} sx={() => ({
            backgroundColor: (index % 2 == 0) ? primaryTableColor : secondaryTableColor,
        })}>
            <Stack>
                <Text>{day}</Text>

                <Divider size="xs"/>

                {getReservationsForDay(index).map((reservation) => {
                    return (<ReservationItem {...reservation} key={reservation.id}/>);
                })}
            </Stack>
        </Grid.Col>
    ));

    return (<>
        <Head>
            <title>Rezervari</title>
            <meta name="viewport" content="initial-scale=1.0, width=device-width"/>
        </Head>

        <Title>Rezervari</Title>

        <Space h="xl"/>

        <Paper>
            <Grid columns={(days.length + 1) * 2 - 1} styles={{"width": "100%"}}>
                <Grid.Col span={1}>
                    <Stack>
                        <Text>Ora</Text>
                    </Stack>
                </Grid.Col>

                {rows}
            </Grid>
        </Paper>
    </>);
}

export async function getStaticProps({}) {
    const data = [
        // new Reservation(1, 10, "2020-03-15", 8 * 60 + 20, 10 * 60 + 15, 2),
        // new Reservation(2, 3, "2020-03-14", 9 * 60 + 10, 11 * 60 + 20, 1),
    ];

    return {
        props: {list: JSON.parse(JSON.stringify(data))},
    }
}
